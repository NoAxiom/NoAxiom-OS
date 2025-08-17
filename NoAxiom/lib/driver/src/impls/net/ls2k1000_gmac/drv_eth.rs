//! eth_init
//! eth_tx
//! eth_rx
//! eth_irq

#![allow(dead_code, unused_assignments, unused_mut)]

use alloc::vec::Vec;

use super::{eth_defs::*, eth_dev::*, platform::*};

// 检查rgmii链路状态
// eth_update_linkstate通知操作系统链路状态
pub fn eth_phy_rgsmii_check(gmacdev: &mut LsGmacInner) {
    let mut value: u32 = 0;
    let mut status: u32 = 0;

    value = eth_mac_read_reg(gmacdev.MacBase, GmacRgsmiiStatus);
    status = value & (MacLinkStatus >> MacLinkStatusOff);

    if gmacdev.LinkStatus != status {
        eth_update_linkstate(gmacdev, status);
    }

    if status != 0 {
        gmacdev.LinkStatus = 1;
        gmacdev.DuplexMode = value & MacLinkMode;
        let mut speed: u32 = value & MacLinkSpeed;
        if speed == MacLinkSpeed_125 {
            gmacdev.Speed = 1000;
        } else if speed == MacLinkSpeed_25 {
            gmacdev.Speed = 100;
        } else {
            gmacdev.Speed = 10;
        }
        log::info!(
            "Link is Up - {} Mpbs / {} Duplex",
            gmacdev.Speed,
            if gmacdev.DuplexMode != 0 {
                "Full"
            } else {
                "Half"
            },
        );
    } else {
        gmacdev.LinkStatus = 0;
        log::info!("Link is Down");
    };
}

// 初始化phy
pub fn eth_phy_init(gmacdev: &mut LsGmacInner) {
    let mut phy: u32 = 0;
    let mut data: u32 = 0;

    data = eth_mdio_read(gmacdev.MacBase, gmacdev.PhyBase as u32, 2) as u32;
    phy |= data << 16;
    data = eth_mdio_read(gmacdev.MacBase, gmacdev.PhyBase as u32, 3) as u32;
    phy |= data;

    match phy {
        0x0000010a => {
            log::info!("probed ethernet phy YT8511H/C, id {:#x}", phy,);
        }
        _ => {
            log::info!("probed unknown ethernet phy, id {:#x}", phy,);
        }
    };
}

pub fn eth_handle_tx_over(gmacdev: &mut LsGmacInner) {
    loop {
        let mut desc_idx: u32 = gmacdev.TxBusy;
        let mut txdesc: DmaDesc = unsafe { gmacdev.TxDesc[desc_idx as usize].read() } as DmaDesc;

        if eth_get_desc_owner(&txdesc) || eth_is_desc_empty(&txdesc) {
            break;
        }

        if eth_is_tx_desc_valid(&txdesc) {
            let mut length: u32 = (txdesc.length & DescSize1Mask) >> DescSize1Shift;
            gmacdev.tx_bytes += length as u64;
            gmacdev.tx_packets += 1;
        } else {
            gmacdev.tx_errors += 1;
        }

        let is_last: bool = eth_is_last_tx_desc(&txdesc);
        txdesc.status = if is_last { TxDescEndOfRing } else { 0 };
        txdesc.length = 0;
        txdesc.buffer1 = 0;
        txdesc.buffer2 = 0;
        unsafe {
            gmacdev.TxDesc[desc_idx as usize].write(txdesc);
        }

        gmacdev.TxBusy = if is_last { 0 } else { desc_idx + 1 };
    }
}

pub fn eth_tx_can_send(gmacdev: &mut LsGmacInner) -> bool {
    let mut desc_idx: u32 = gmacdev.TxNext;
    let mut txdesc: DmaDesc = unsafe { gmacdev.TxDesc[desc_idx as usize].read() } as DmaDesc;
    !eth_get_desc_owner(&txdesc)
}

// 操作系统传递接收数据的单元pbuf给驱动
// pbuf可能是操作系统自定义结构
// 返回接收到的数据字节数
pub fn eth_tx(gmacdev: &mut LsGmacInner, pbuf: &[u8]) -> i32 {
    let mut buffer: u64 = 0;
    let mut length: u32 = 0;
    let mut dma_addr: u32 = 0;
    let mut desc_idx: u32 = gmacdev.TxNext;
    let mut txdesc: DmaDesc = unsafe { gmacdev.TxDesc[desc_idx as usize].read() } as DmaDesc;
    let mut is_last: bool = eth_is_last_tx_desc(&txdesc);

    if eth_get_desc_owner(&txdesc) {
        return -1;
    }

    buffer = gmacdev.TxBuffer[desc_idx as usize];
    length = eth_handle_tx_buffer(pbuf, buffer);
    dma_addr = eth_virt_to_phys(buffer);

    txdesc.status |= DescOwnByDma | DescTxIntEnable | DescTxLast | DescTxFirst;
    txdesc.length = length << DescSize1Shift & DescSize1Mask;
    txdesc.buffer1 = dma_addr;
    txdesc.buffer2 = 0;
    unsafe {
        gmacdev.TxDesc[desc_idx as usize].write(txdesc);
    }

    gmacdev.TxNext = if is_last { 0 } else { desc_idx + 1 };

    eth_sync_dcache();

    eth_gmac_resume_dma_tx(gmacdev);

    return 0;
}

// pbuf是返回给操作系统的数据单元
// 可能是操作系统自定义结构
pub fn eth_rx(gmacdev: &mut LsGmacInner) -> Option<Vec<u8>> {
    let mut desc_idx: u32 = gmacdev.RxBusy;
    let mut rxdesc: DmaDesc = unsafe { gmacdev.RxDesc[desc_idx as usize].read() } as DmaDesc;
    let mut is_last: bool = eth_is_last_rx_desc(&rxdesc);

    if eth_is_desc_empty(&rxdesc) || eth_get_desc_owner(&rxdesc) {
        eth_dma_enable_interrupt(gmacdev, DmaIntEnable);
        return None;
    }

    let mut pbuf = None;
    let mut dma_addr = rxdesc.buffer1;

    if eth_is_rx_desc_valid(&rxdesc) {
        let mut length: u32 = eth_get_rx_length(&rxdesc);
        let mut buffer: u64 = eth_phys_to_virt(dma_addr);

        eth_sync_dcache();

        pbuf = Some(eth_handle_rx_buffer(buffer, length));
        gmacdev.rx_bytes += length as u64;
        gmacdev.rx_packets += 1;
    } else {
        gmacdev.rx_errors += 1;
    }

    rxdesc.status = DescOwnByDma;
    rxdesc.length = if is_last { RxDescEndOfRing } else { 0 };
    rxdesc.length |= (2048) << DescSize1Shift & DescSize1Mask;
    rxdesc.buffer1 = dma_addr;
    rxdesc.buffer2 = 0;
    unsafe {
        gmacdev.RxDesc[desc_idx as usize].write(rxdesc);
    }

    gmacdev.RxBusy = if is_last { 0 } else { desc_idx + 1 };
    pbuf
}

// 中断处理程序
// eth_rx_ready通知操作系统可以接收数据
// eth_handle_tx_over用于处理已经发送完的描述符
pub fn eth_irq(gmacdev: &mut LsGmacInner) -> bool {
    let mut dma_status: u32 = 0;
    let mut dma_int_enable: u32 = DmaIntEnable;

    dma_status = eth_mac_read_reg(gmacdev.DmaBase, DmaStatus);
    if dma_status == 0 {
        return false;
    }

    eth_dma_disable_interrupt_all(gmacdev);

    if dma_status & GmacPmtIntr != 0 {
        log::error!("gmac pmt interrupt");
    }
    if dma_status & GmacMmcIntr != 0 {
        log::error!("gmac mmc interrupt");
    }
    if dma_status & GmacLineIntfIntr != 0 {
        eth_mac_read_reg(gmacdev.MacBase, GmacInterruptStatus);
        eth_mac_read_reg(gmacdev.MacBase, GmacInterruptMask);
        if eth_mac_read_reg(gmacdev.MacBase, GmacInterruptStatus) & GmacRgmiiIntSts != 0 {
            eth_mac_read_reg(gmacdev.MacBase, GmacRgsmiiStatus);
        }
        eth_phy_rgsmii_check(gmacdev);
    }

    eth_mac_write_reg(gmacdev.DmaBase, DmaStatus, dma_status);

    if dma_status & DmaIntBusError != 0 {
        log::error!("gmac fatal bus error interrupt");
    }
    if dma_status & DmaIntRxStopped != 0 {
        log::error!("gmac receive process stopped");
        eth_dma_enable_rx(gmacdev);
    }
    if dma_status & DmaIntRxNoBuffer != 0 {
        log::error!("gmac receive buffer unavailable");
        dma_int_enable &= !DmaIntRxNoBuffer;
        eth_gmac_resume_dma_rx(gmacdev);
        eth_rx_ready(gmacdev);
    }
    if dma_status & DmaIntRxCompleted != 0 {
        dma_int_enable &= !DmaIntRxCompleted;
        eth_rx_ready(gmacdev);
    }
    if dma_status & DmaIntTxUnderflow != 0 {
        log::error!("gmac transmit underflow");
    }
    if dma_status & DmaIntRcvOverflow != 0 {
        log::error!("gmac receive underflow");
    }
    if dma_status & DmaIntTxNoBuffer != 0 {}
    if dma_status & DmaIntTxStopped != 0 {
        log::error!("gmac transmit process stopped");
    }
    if dma_status & DmaIntTxCompleted != 0 {
        eth_handle_tx_over(gmacdev);
    }
    eth_dma_enable_interrupt(gmacdev, dma_int_enable);

    return true;
}

// 初始化
pub fn eth_init(gmacdev: &mut LsGmacInner) -> i32 {
    log::info!("Initializing GMAC...");

    // 在eth_init内或外，利用uncached地址初始化结构体的iobase
    gmacdev.iobase = eth_phys_to_uncached(0x40040000);
    gmacdev.MacBase = gmacdev.iobase + 0x0000;
    gmacdev.DmaBase = gmacdev.iobase + 0x1000;
    gmacdev.PhyBase = 0;
    gmacdev.Version = eth_mac_read_reg(gmacdev.MacBase, GmacVersion);

    eth_dma_reset(gmacdev);
    eth_mac_set_addr(gmacdev);
    eth_phy_init(gmacdev);

    eth_setup_rx_desc_queue(gmacdev, 128);
    eth_setup_tx_desc_queue(gmacdev, 128);

    eth_dma_reg_init(gmacdev);
    eth_gmac_reg_init(gmacdev);

    eth_sync_dcache();

    eth_gmac_disable_mmc_irq(gmacdev);
    eth_dma_clear_curr_irq(gmacdev);
    eth_dma_enable_interrupt(gmacdev, DmaIntEnable);

    eth_gmac_enable_rx(gmacdev);
    eth_gmac_enable_tx(gmacdev);
    eth_dma_enable_rx(gmacdev);
    eth_dma_enable_tx(gmacdev);

    eth_isr_install();

    return 0;
}
