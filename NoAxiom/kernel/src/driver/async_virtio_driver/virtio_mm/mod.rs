//! qemu virtio 前端驱动
use alloc::{sync::Arc, vec::Vec};

use async_blk::VirtIOAsyncBlock;
use lazy_static::*;
use spin::Mutex;

use super::dma::VirtualAddress;
use crate::{
    driver::async_virtio_driver::dma::PhysicalAddress,
    mm::{
        address::{PhysAddr, PhysPageNum, VirtAddr},
        frame::{frame_alloc, FrameTracker},
        memory_set::KERNEL_SPACE,
        page_table::PageTable,
    },
    println,
    utils::{kernel_pa_to_va, kernel_va_to_pa},
};
pub mod async_blk;

lazy_static! {
    static ref QUEUE_FRAMES: Mutex<Vec<FrameTracker>> = Mutex::new(Vec::new());
    pub static ref VIRTIO_BLOCK: Arc<VirtIOAsyncBlock> = Arc::new(VirtIOAsyncBlock::new());
}

// 提供给`async-virtio-driver`的函数
pub fn virtio_dma_alloc(pages: usize) -> PhysicalAddress {
    let mut ppn_base = 0;
    for i in 0..pages {
        let frame = frame_alloc();
        if i == 0 {
            ppn_base = frame.ppn().into();
        }
        let frame_ppn: usize = frame.ppn().into();
        assert_eq!(frame_ppn, ppn_base + i);
        QUEUE_FRAMES.lock().push(frame);
    }
    PhysAddr::from(PhysPageNum::from(ppn_base)).into()
}

// 提供给`async-virtio-driver`的函数
// todo: 检查这里
pub fn virtio_dma_dealloc(pa: PhysicalAddress, pages: usize) -> i32 {
    let ppn = PhysPageNum::from(PhysAddr::from(pa));
    let mut remove_idx = -1;
    let mut q = QUEUE_FRAMES.lock();
    for (idx, frame) in q.iter().enumerate() {
        if frame.ppn() == ppn {
            remove_idx = idx as i32;
        }
    }
    if remove_idx != -1 {
        for _ in 0..pages {
            let pop_frame = q.remove(remove_idx as usize);
            // 最终会调用 FrameTracker::drop()，在帧分配器中释放持有的帧内存
            drop(pop_frame);
        }
    } else {
        return -1;
    }
    0
}

// 提供给`async-virtio-driver`的函数
// 这里可以直接使用线性映射的关系
pub fn virtio_phys_to_virt(paddr: PhysicalAddress) -> VirtualAddress {
    VirtualAddress::from(kernel_pa_to_va(paddr))
}

// 提供给`async-virtio-driver`的函数
// 这里需要查页表
// todo: why?
pub fn virtio_virt_to_phys(vaddr: VirtualAddress) -> PhysicalAddress {
    // let offset = vaddr.get_bits(0..12); // Sv39 低 12 位是页内偏移
    // let satp = Satp(satp::read().bits());
    // let vpn = VirtPageNum::from(VirtAddr::from(vaddr));
    // let ppn = satp
    //     .translate(vpn)
    //     .expect("virtio virtual address not map!");
    // ppn.start_address().add(offset)
    let pa = PhysicalAddress::from(kernel_va_to_pa(vaddr));
    let translated_pa = PageTable::from_token(KERNEL_SPACE.lock().token())
        .translate_va(VirtAddr::from(vaddr))
        .unwrap()
        .into();
    assert_eq!(pa, translated_pa, "virtio_virt_to_phys translation failed");
    pa
}

/// 异步virtio块设备驱动测试
#[allow(unused)]
pub async fn async_virtio_blk_test() {
    let mut read_buf = [0u8; 512];
    let mut write_buf = [0u8; 512];
    for i in 0..512 {
        write_buf.iter_mut().for_each(|byte| *byte = i as u8);
        VIRTIO_BLOCK.write_block(i as usize, &write_buf).await;
        VIRTIO_BLOCK.read_block(i as usize, &mut read_buf).await;
        assert_eq!(read_buf, write_buf);
    }
    println!("[kernel] async_virtio_blk_test pass");
}
