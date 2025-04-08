use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use core::task::Waker;

use arch::MappingFlags;
use ksync::mutex::{SpinLock, SpinLockGuard};

use super::{
    address::{VirtAddr, VirtPageNum, VpnRange},
    frame::{frame_alloc, FrameTracker},
    memory_set::MemorySet,
};
use crate::{
    config::mm::{MMAP_BASE_ADDR, PAGE_SIZE},
    cpu::current_cpu,
    fs::vfs::basic::file::File,
    include::{
        mm::{MmapFlags, MmapProts},
        result::Errno,
    },
    sched::utils::{suspend_no_int_now, take_waker},
    syscall::SysResult,
};

/// single mmap page struct
#[derive(Clone)]
pub struct MmapPage {
    /// base va of mmap space
    pub vpn: VirtPageNum,

    /// mmap protection
    pub prot: MmapProts,

    /// mmap flags
    pub flags: MmapFlags,

    /// validity, indicating whether the page is acutally mapped
    pub valid: bool,

    /// mmapped file
    pub file: Option<Arc<dyn File>>,

    /// offset in file
    pub offset: usize,
}

impl MmapPage {
    /// mmap alloc
    pub async fn lazy_map_page(&mut self, kernel_vpn: VirtPageNum) -> SysResult<()> {
        if let Some(file) = self.file.clone() {
            let buf_slice: &mut [u8] = unsafe {
                core::slice::from_raw_parts_mut(kernel_vpn.as_va_usize() as *mut u8, PAGE_SIZE)
            };
            // crate::sched::utils::yield_now().await;
            let res = file.base_read(self.offset, buf_slice).await;
            if let Err(res) = res {
                error!("ERROR at mmap read file, msg: {:?}", res);
            }
        }
        self.valid = true;
        Ok(())
    }
}

pub struct MmapManager {
    /// base of mmap space
    pub mmap_start: VirtAddr,

    /// top of mmap space
    pub mmap_top: VirtAddr,

    /// mmap space, containing all mmap pages whenever they are allocated or not
    pub mmap_map: BTreeMap<VirtPageNum, MmapPage>,

    /// frame trackers for already allocated mmap pages
    pub frame_trackers: BTreeMap<VirtPageNum, FrameTracker>,

    /// mmap alloc tracer
    pub alloc_tracer: BTreeMap<VirtPageNum, Vec<Waker>>,
}

impl MmapManager {
    pub fn new(mmap_start: VirtAddr, mmap_top: VirtAddr) -> Self {
        Self {
            mmap_start,
            mmap_top,
            mmap_map: BTreeMap::new(),
            frame_trackers: BTreeMap::new(),
            alloc_tracer: BTreeMap::new(),
        }
    }

    pub fn new_bare() -> Self {
        Self::new(VirtAddr(MMAP_BASE_ADDR), VirtAddr(MMAP_BASE_ADDR))
    }

    /// push a mmap range in mmap space (not actually mapped)
    pub fn insert(
        &mut self,
        start_va: VirtAddr,
        length: usize,
        prot: MmapProts,
        flags: MmapFlags,
        st_offset: usize,
        file: Option<Arc<dyn File>>,
    ) -> usize {
        let end_va = VirtAddr(start_va.0 + length);
        let mut offset = st_offset;
        for vpn in VpnRange::new_from_va(start_va, end_va) {
            // created a mmap page with lazy-mapping
            let mmap_page = MmapPage {
                vpn,
                prot,
                flags,
                valid: false,
                file: file.clone(),
                offset,
            };
            self.mmap_map.insert(vpn, mmap_page);
            offset += PAGE_SIZE;
        }
        if self.mmap_top <= start_va {
            self.mmap_top = (start_va.0 + length).into();
        }
        start_va.0
    }

    /// remove a mmap range in mmap space
    pub fn remove(&mut self, start_va: VirtAddr, length: usize) {
        let end_va = VirtAddr(start_va.0 + length);
        for vpn in VpnRange::new_from_va(start_va, end_va) {
            self.mmap_map.remove(&vpn);
            self.frame_trackers.remove(&vpn);
        }
    }

    /// is a va in mmap space
    pub fn is_in_space(&self, vpn: VirtPageNum) -> bool {
        self.mmap_map.contains_key(&vpn) || self.alloc_tracer.contains_key(&vpn)
    }
}

pub async fn lazy_alloc_mmap<'a>(
    memory_set: &Arc<SpinLock<MemorySet>>,
    vpn: VirtPageNum,
    mut guard: SpinLockGuard<'a, MemorySet>,
) -> SysResult<()> {
    let frame = frame_alloc();
    let ppn = frame.ppn();
    let kernel_vpn = frame.into_kernel_vpn();
    guard.mmap_manager.frame_trackers.insert(vpn, frame);
    let mmap_page = guard.mmap_manager.mmap_map.remove(&vpn);
    match mmap_page {
        Some(mut mmap_page) => {
            drop(guard);
            let pte_flags: MappingFlags = MappingFlags::from(mmap_page.prot) | MappingFlags::U;
            mmap_page.lazy_map_page(kernel_vpn).await?;
            let mut ms = memory_set.lock();
            ms.page_table().map(vpn, ppn, pte_flags);
            if let Some(tracer) = ms.mmap_manager.alloc_tracer.get_mut(&vpn) {
                for waker in tracer.iter() {
                    waker.wake_by_ref();
                }
                ms.mmap_manager.alloc_tracer.remove(&vpn);
            }
            assert!(ms.mmap_manager.mmap_map.get(&vpn).is_none());
            assert!(ms.mmap_manager.alloc_tracer.get(&vpn).is_none());
            ms.mmap_manager.mmap_map.insert(vpn, mmap_page);
            drop(ms);
            Ok(())
        }
        None => match guard.mmap_manager.alloc_tracer.get_mut(&vpn) {
            Some(tracer) => {
                tracer.push(take_waker().await);
                drop(guard);
                debug!("[lazy_alloc_mmap] suspend_no_int_now");
                loop {
                    suspend_no_int_now(current_cpu().task.as_ref().unwrap().pcb()).await;
                    if memory_set.lock().mmap_manager.mmap_map.get(&vpn).is_some() {
                        break;
                    }
                }
                Ok(())
            }
            None => {
                error!("[lazy_alloc_mmap] vpn not found in mmap_map");
                Err(Errno::EFAULT)
            }
        },
    }
}

/*

几个很麻烦的东西

mmap这玩意可能会读取文件信息
我不想把这个lazymmap做成blockon的行为
它会先让权，过一会儿再回来把数据放进去
那么这个读取文件期间，我们是没有对于memoryset进行lock的
这个时候会出现munmap发生
那么就需要进入memoryset.lock再次检查当前mmap区间的合法性

目前打算搞一个妥协的方案, 只在kernel_trap的时候进行block_on的行为

此外munmap的时候需要进行tlb shootdown，防止往已经dealloc的区间进行数据的写入
这里需要对于IPI进行维护
鉴于我们已经使用IPI进行了多核负载均衡的请求，还需要额外添加IPI_info的维护

不过我觉得mmap的tlb shootdown没有很大的必要诶？？
因为这玩意其实并不影响正确性，只是会影响信息到达的时间
到底要不要发ipi啊 =^=

pub struct LazyAllocMmapFuture<'a> {
    memory_set: &'a Arc<SpinLock<MemorySet>>,
    vpn: VirtPageNum,
    mmap_page: MmapPage,
}
impl<'a> LazyAllocMmapFuture<'a> {
    pub fn new(
        memory_set: &'a Arc<SpinLock<MemorySet>>,
        vpn: VirtPageNum,
        mmap_page: MmapPage,
    ) -> Self {
        Self {
            memory_set,
            vpn,
            mmap_page,
        }
    }
}
impl<'a> Future for LazyAllocMmapFuture<'a> {
    type Output = SysResult<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
{         let this = self.get_mut();
        let mut fut = this.mmap_page.lazy_map_page();
        let pin = pin!(fut);
        let res = pin.poll(cx);
        if res.is_ready() {

        }
        res
    }
}
LazyAllocMmapFuture::new(memory_set, vpn, mmap_page).await?;

*/
