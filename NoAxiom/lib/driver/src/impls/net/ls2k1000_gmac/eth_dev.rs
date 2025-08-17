#![allow(dead_code, unused_assignments, unused_mut)]

use core::ptr::{null_mut, read_volatile, write_volatile};

use super::{eth_defs::*, platform::*};

pub fn eth_mac_read_reg(base: u64, offset: u32) -> u32 {
    let mut addr: u64 = 0;
    let mut data: u32 = 0;
    addr = base + offset as u64;
    unsafe { data = read_volatile(addr as *mut u32) };
    return data;
}

pub fn eth_mac_write_reg(mut base: u64, mut offset: u32, mut data: u32) {
    let mut addr: u64;
    addr = base + offset as u64;
    unsafe { write_volatile(addr as *mut u32, data) };
}

pub fn eth_mac_set_bits(base: u64, offset: u32, pos: u32) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(base, offset);
    data |= pos;
    eth_mac_write_reg(base, offset, data);
}

pub fn eth_mac_clear_bits(base: u64, offset: u32, pos: u32) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(base, offset);
    data &= !pos;
    eth_mac_write_reg(base, offset, data);
}

pub fn eth_mdio_read(regbase: u64, phybase: u32, offset: u32) -> u16 {
    let mut addr: u32 = 0;
    addr = (phybase << GmiiDevShift) & GmiiDevMask | (offset << GmiiRegShift) & GmiiRegMask;
    addr |= GmiiCsrClk4 | GmiiBusy;

    eth_mac_write_reg(regbase, GmacGmiiAddr, addr);
    while eth_mac_read_reg(regbase, GmacGmiiAddr) & GmiiBusy != 0 {}

    return (eth_mac_read_reg(regbase, GmacGmiiData) & 0xffff) as u16;
}

pub fn eth_mdio_write(regbase: u64, phybase: u32, offset: u32, data: u16) {
    eth_mac_write_reg(regbase, GmacGmiiData, data as u32);

    let mut addr: u32 = 0;
    addr = phybase << GmiiDevShift & GmiiDevMask | offset << GmiiRegShift & GmiiRegMask;
    addr |= GmiiWrite | GmiiCsrClk4 | GmiiBusy;

    eth_mac_write_reg(regbase, GmacGmiiAddr, addr);
    while eth_mac_read_reg(regbase, GmacGmiiAddr) & GmiiBusy != 0 {}
}

pub fn eth_mac_set_addr(gmacdev: &LsGmacInner) {
    let addr: [u8; 6] = gmacdev.MacAddr;
    let mut data: u32;

    data = ((addr[5] as u32) << 8) | (addr[4] as u32);
    eth_mac_write_reg(gmacdev.MacBase, GmacAddr0High, data);

    data = ((addr[3] as u32) << 24)
        | ((addr[2] as u32) << 16)
        | ((addr[1] as u32) << 8)
        | (addr[0] as u32);
    eth_mac_write_reg(gmacdev.MacBase, GmacAddr0Low, data);
}

pub fn eth_gmac_get_mac_addr(gmacdev: &LsGmacInner, addr: &mut [u8; 6]) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.MacBase, GmacAddr0High);
    addr[5] = ((data >> 8) & 0xff) as u8;
    addr[4] = (data & 0xff) as u8;

    data = eth_mac_read_reg(gmacdev.MacBase, GmacAddr0Low);
    addr[3] = ((data >> 24) & 0xff) as u8;
    addr[2] = ((data >> 16) & 0xff) as u8;
    addr[1] = ((data >> 8) & 0xff) as u8;
    addr[0] = (data & 0xff) as u8;
}

pub fn eth_dma_reset(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;

    eth_mac_write_reg(gmacdev.DmaBase, DmaBusMode, DmaResetOn);

    loop {
        data = eth_mac_read_reg(gmacdev.DmaBase, DmaBusMode);
        if (data & 1) == 0 {
            break;
        }
    }
}

pub fn eth_gmac_resume_dma_rx(gmacdev: &LsGmacInner) {
    eth_mac_write_reg(gmacdev.DmaBase, DmaRxPollDemand, 0);
}

pub fn eth_gmac_resume_dma_tx(gmacdev: &LsGmacInner) {
    eth_mac_write_reg(gmacdev.DmaBase, DmaTxPollDemand, 0);
}

pub fn eth_dma_enable_rx(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.DmaBase, DmaControl);
    data |= DmaRxStart;
    eth_mac_write_reg(gmacdev.DmaBase, DmaControl, data);
}

pub fn eth_dma_enable_tx(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.DmaBase, DmaControl);
    data |= DmaTxStart;
    eth_mac_write_reg(gmacdev.DmaBase, DmaControl, data);
}

pub fn eth_gmac_disable_dma_tx(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.DmaBase, DmaControl);
    data &= !DmaTxStart;
    eth_mac_write_reg(gmacdev.DmaBase, DmaControl, data);
}

pub fn eth_gmac_disable_dma_rx(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.DmaBase, DmaControl);
    data &= !DmaRxStart;
    eth_mac_write_reg(gmacdev.DmaBase, DmaControl, data);
}

pub fn eth_gmac_enable_rx(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.MacBase, GmacConfig);
    data |= GmacRx;
    eth_mac_write_reg(gmacdev.MacBase, GmacConfig, data);
}

pub fn eth_gmac_enable_tx(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.MacBase, GmacConfig);
    data |= GmacTx;
    eth_mac_write_reg(gmacdev.MacBase, GmacConfig, data);
}

pub fn eth_gmac_disable_rx(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.MacBase, GmacConfig);
    data &= !GmacRx;
    eth_mac_write_reg(gmacdev.MacBase, GmacConfig, data);
}

pub fn eth_gmac_disable_tx(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.MacBase, GmacConfig);
    data &= !GmacTx;
    eth_mac_write_reg(gmacdev.MacBase, GmacConfig, data);
}

pub fn eth_dma_clear_curr_irq(gmacdev: &LsGmacInner) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.DmaBase, DmaStatus);
    eth_mac_write_reg(gmacdev.DmaBase, DmaStatus, data);
}

pub fn eth_dma_clear_irq(gmacdev: &LsGmacInner, value: u32) {
    eth_mac_write_reg(gmacdev.DmaBase, DmaStatus, value);
}

pub fn eth_dma_enable_interrupt(gmacdev: &LsGmacInner, value: u32) {
    eth_mac_write_reg(gmacdev.DmaBase, DmaInterrupt, value);
}

pub fn eth_dma_disable_interrupt_all(gmacdev: &LsGmacInner) {
    eth_mac_write_reg(gmacdev.DmaBase, DmaInterrupt, DmaIntDisable);
}

pub fn eth_dma_disable_interrupt(gmacdev: &LsGmacInner, value: u32) {
    eth_mac_clear_bits(gmacdev.DmaBase, DmaInterrupt, value);
}

pub fn eth_gmac_disable_mmc_irq(gmacdev: &LsGmacInner) {
    eth_mac_write_reg(gmacdev.MacBase, GmacMmcIntrMaskTx, 0xffffffff);
    eth_mac_write_reg(gmacdev.MacBase, GmacMmcIntrMaskRx, 0xffffffff);
    eth_mac_write_reg(gmacdev.MacBase, GmacMmcRxIpcIntrMask, 0xffffffff);
}

pub fn eth_dma_bus_mode_init(gmacdev: &LsGmacInner) {
    let mut value: u32 = 0;
    value |= DmaMixedBurstEnable;
    value |= DmaBurstLengthx8 | DmaBurstLength32;
    value |= DmaDescriptor4DWords | DmaDescriptorSkip0;
    eth_mac_write_reg(gmacdev.DmaBase, DmaBusMode, value);
}

pub fn eth_dma_control_init(gmacdev: &LsGmacInner) {
    let mut value: u32 = 0;
    value |= DmaStoreAndForward | DmaTxSecondFrame;
    eth_mac_write_reg(gmacdev.DmaBase, DmaControl, value);
}

pub fn eth_dma_axi_bus_mode_init(gmacdev: &LsGmacInner) {
    let mut value: u32 = 0;
    value |= 0xff;
    value |= 0x77 << 16;
    eth_mac_write_reg(gmacdev.DmaBase, 0x28, value);
}

pub fn eth_dma_reg_init(gmacdev: &LsGmacInner) {
    eth_dma_bus_mode_init(gmacdev);
    eth_dma_control_init(gmacdev);
    eth_dma_axi_bus_mode_init(gmacdev);
}

pub fn eth_gmac_back_off_limit(gmacdev: &LsGmacInner, value: u32) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.MacBase, GmacConfig);
    data &= !GmacBackoffLimit;
    data |= value;
    eth_mac_write_reg(gmacdev.MacBase, GmacConfig, data);
}

pub fn eth_gmac_config_init(gmacdev: &LsGmacInner) {
    eth_mac_set_bits(gmacdev.MacBase, GmacConfig, GmacTxConfig);
    eth_mac_clear_bits(gmacdev.MacBase, GmacConfig, GmacWatchdog);
    eth_mac_clear_bits(gmacdev.MacBase, GmacConfig, GmacJabber);
    eth_mac_clear_bits(gmacdev.MacBase, GmacConfig, GmacFrameBurst);
    eth_mac_clear_bits(gmacdev.MacBase, GmacConfig, GmacJumboFrame);
    eth_mac_clear_bits(gmacdev.MacBase, GmacConfig, GmacRxOwn);
    eth_mac_clear_bits(gmacdev.MacBase, GmacConfig, GmacLoopback);
    eth_mac_set_bits(gmacdev.MacBase, GmacConfig, GmacDuplex);
    eth_mac_clear_bits(gmacdev.MacBase, GmacConfig, GmacRetry);
    eth_mac_clear_bits(gmacdev.MacBase, GmacConfig, GmacPadCrcStrip);
    eth_mac_clear_bits(gmacdev.MacBase, GmacConfig, GmacDeferralCheck);
    eth_gmac_back_off_limit(gmacdev, GmacBackoffLimit0);
}

pub fn eth_gmac_set_pass_control(gmacdev: &LsGmacInner, value: u32) {
    let mut data: u32 = 0;
    data = eth_mac_read_reg(gmacdev.MacBase, GmacFrameFilter);
    data &= !GmacPassControl;
    data |= value;
    eth_mac_write_reg(gmacdev.MacBase, GmacFrameFilter, data);
}

pub fn eth_gmac_frame_filter(gmacdev: &LsGmacInner) {
    eth_mac_clear_bits(gmacdev.MacBase, GmacFrameFilter, GmacSrcAddrFilter);
    eth_gmac_set_pass_control(gmacdev, GmacPassControl0);
    eth_mac_clear_bits(gmacdev.MacBase, GmacFrameFilter, GmacBroadcast);
    eth_mac_clear_bits(gmacdev.MacBase, GmacFrameFilter, GmacMulticastFilter);
    eth_mac_clear_bits(gmacdev.MacBase, GmacFrameFilter, GmacDestAddrFilter);
    eth_mac_clear_bits(gmacdev.MacBase, GmacFrameFilter, GmacMcastHashFilter);
    eth_mac_clear_bits(gmacdev.MacBase, GmacFrameFilter, GmacUcastHashFilter);
    eth_mac_clear_bits(gmacdev.MacBase, GmacFrameFilter, GmacPromiscuousMode);
    eth_mac_set_bits(gmacdev.MacBase, GmacFrameFilter, GmacFilter);
}

pub fn eth_gmac_flow_control(gmacdev: &LsGmacInner) {
    let mut dma_ctrl: u32 = 0;
    dma_ctrl = eth_mac_read_reg(gmacdev.DmaBase, DmaControl);
    dma_ctrl &= !(DmaRxFlowCtrlAct | DmaRxFlowCtrlDeact);
    dma_ctrl &= !DmaEnHwFlowCtrl;
    eth_mac_write_reg(gmacdev.DmaBase, DmaControl, dma_ctrl);

    let mut flow_ctrl: u32 = 0;
    flow_ctrl |= GmacPauseTimeMask;
    flow_ctrl &= !(GmacRxFlowControl | GmacTxFlowControl);
    eth_mac_write_reg(gmacdev.MacBase, GmacFlowControl, flow_ctrl);
}

pub fn eth_gmac_reg_init(gmacdev: &LsGmacInner) {
    eth_gmac_config_init(gmacdev);
    eth_gmac_frame_filter(gmacdev);
    eth_gmac_flow_control(gmacdev);
}

pub fn eth_setup_tx_desc_queue(gmacdev: &mut LsGmacInner, desc_num: u32) {
    let mut desc: *mut DmaDesc = null_mut();
    let mut dma_addr: u32 = 0;
    let mut buffer: u64 = 0;

    desc = eth_malloc_align(
        (core::mem::size_of::<DmaDesc>() * (desc_num as usize)) as u64,
        16,
    ) as *mut DmaDesc;
    dma_addr = eth_virt_to_phys(desc as u64);

    gmacdev.TxNext = 0;
    gmacdev.TxBusy = 0;

    eth_mac_write_reg(gmacdev.DmaBase, DmaTxBaseAddr, dma_addr);

    for i in 0..desc_num {
        buffer = eth_malloc_align(2048, 16);
        gmacdev.TxDesc[i as usize] = desc;
        gmacdev.TxBuffer[i as usize] = buffer;

        let is_last = i == desc_num - 1;
        unsafe {
            (*desc).status = if is_last { TxDescEndOfRing } else { 0 };
            (*desc).length = 0;
            (*desc).buffer1 = 0;
            (*desc).buffer2 = 0;
            desc = desc.offset(1);
        }
    }
}

pub fn eth_setup_rx_desc_queue(gmacdev: &mut LsGmacInner, desc_num: u32) {
    let mut desc: *mut DmaDesc = null_mut();
    let mut dma_addr: u32 = 0;
    let mut buffer: u64 = 0;

    desc = eth_malloc_align(
        (core::mem::size_of::<DmaDesc>() * (desc_num as usize)) as u64,
        16,
    ) as *mut DmaDesc;
    dma_addr = eth_virt_to_phys(desc as u64);

    gmacdev.RxBusy = 0;

    eth_mac_write_reg(gmacdev.DmaBase, DmaRxBaseAddr, dma_addr);

    for i in 0..desc_num {
        buffer = eth_malloc_align(2048, 16);
        dma_addr = eth_virt_to_phys(buffer);
        gmacdev.RxDesc[i as usize] = desc;
        gmacdev.RxBuffer[i as usize] = buffer;

        let is_last = i == desc_num - 1;
        unsafe {
            (*desc).status = DescOwnByDma;
            (*desc).length = if is_last { RxDescEndOfRing } else { 0 };
            (*desc).length |= ((2048 << DescSize1Shift) & DescSize1Mask) as u32;
            (*desc).buffer1 = dma_addr;
            (*desc).buffer2 = 0;
            desc = desc.offset(1);
        }
    }
}

pub fn eth_get_desc_owner(desc: &DmaDesc) -> bool {
    return (desc.status & DescOwnByDma) == DescOwnByDma;
}

pub fn eth_get_rx_length(desc: &DmaDesc) -> u32 {
    return (desc.status & DescFrameLengthMask) >> DescFrameLengthShift;
}

pub fn eth_is_tx_desc_valid(desc: &DmaDesc) -> bool {
    return (desc.status & DescError) == 0;
}

pub fn eth_is_desc_empty(desc: &DmaDesc) -> bool {
    return (desc.length & DescSize1Mask == 0) && (desc.length & DescSize2Mask == 0);
}

pub fn eth_is_rx_desc_valid(desc: &DmaDesc) -> bool {
    return (desc.status & DescError == 0)
        && (desc.status & DescRxFirst == DescRxFirst)
        && (desc.status & DescRxLast == DescRxLast);
}

pub fn eth_is_last_rx_desc(desc: &DmaDesc) -> bool {
    return desc.length & RxDescEndOfRing == RxDescEndOfRing;
}

pub fn eth_is_last_tx_desc(desc: &DmaDesc) -> bool {
    return desc.status & TxDescEndOfRing == TxDescEndOfRing;
}
