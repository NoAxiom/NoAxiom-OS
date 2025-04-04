//! qemu virtio 前端驱动

use arch::ArchMemory;

use super::dma::{PhysicalAddress, VirtualAddress};

pub mod async_blk;

// 提供给`async-virtio-driver`的函数
pub fn virtio_dma_alloc(pages: usize) -> PhysicalAddress {
    /*
    lazy_static! {
        static mut ref QUEUE_FRAMES: Vec<FrameTracker> = Vec::new();
    }

    pub fn virtio_dma_alloc(pages: usize) -> PhysicalAddress {
        let mut ppn_base = 0;
        for i in 0..pages {
            let frame = frame_alloc();
            if i == 0 {
                ppn_base = frame.ppn().into();
            }
            let frame_ppn: usize = frame.ppn().into();
            assert_eq!(frame_ppn, ppn_base + i);
            QUEUE_FRAMES.push(frame);
        }
        PhysAddr::from(PhysPageNum::from(ppn_base)).into()
    }
     */
    todo!(" ↑ do like this ↑");
}

// 提供给`async-virtio-driver`的函数
pub fn virtio_dma_dealloc(pa: PhysicalAddress, pages: usize) -> i32 {
    /*
    pub fn virtio_dma_dealloc(pa: PhysicalAddress, pages: usize) -> i32 {
        let ppn = PhysPageNum::from(PhysAddr::from(pa));
        let mut remove_idx = -1;
        let mut q = QUEUE_FRAMES;
        for (idx, frame) in q.iter().enumerate() {
            if frame.ppn() == ppn {
                remove_idx = idx as i32;
            }
        }
        if remove_idx != -1 {
            for _ in 0..pages {
                let pop_frame = q.remove(remove_idx as usize);
                drop(pop_frame);
            }
        } else {
            return -1;
        }
        0
    }
     */
    todo!(" ↑ do like this ↑");
}

// 提供给`async-virtio-driver`的函数
// 这里可以直接使用线性映射的关系
pub fn virtio_phys_to_virt(paddr: PhysicalAddress) -> VirtualAddress {
    VirtualAddress::from(paddr | arch::Arch::KERNEL_ADDR_OFFSET)
}

// 提供给`async-virtio-driver`的函数
pub fn virtio_virt_to_phys(vaddr: VirtualAddress) -> PhysicalAddress {
    PhysicalAddress::from(vaddr & !arch::Arch::KERNEL_ADDR_OFFSET)
}
