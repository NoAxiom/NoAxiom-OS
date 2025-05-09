use alloc::{sync::Arc, vec::Vec};

use arch::{Arch, ArchMemory, ArchPageTableEntry, ArchTime, MappingFlags, PageTableEntry};
use config::{
    fs::ROOT_NAME,
    mm::{DL_INTERP_OFFSET, SIG_TRAMPOLINE, USER_HEAP_SIZE},
};
use include::errno::Errno;
use ksync::{cell::SyncUnsafeCell, mutex::SpinLock};
use spin::Once;
use xmas_elf::ElfFile;

use super::{
    address::{PhysAddr, PhysPageNum},
    frame::{frame_alloc, frame_refcount, FrameTracker},
    map_area::MapArea,
    mmap_manager::MmapManager,
    page_table::{flags_switch_to_rw, PageTable},
    shm::{shm_get_address_and_size, shm_get_nattch, ShmInfo, ShmTracker},
};
use crate::{
    config::mm::{PAGE_SIZE, PAGE_WIDTH, USER_STACK_SIZE},
    fs::{path::Path, vfs::basic::file::File},
    include::process::auxv::*,
    map_permission,
    mm::{
        address::{VirtAddr, VirtPageNum},
        map_area::MapAreaType,
        page_table::flags_switch_to_cow,
        permission::MapType,
    },
    pte_flags,
    sched::utils::yield_now,
    syscall::SysResult,
    task::impl_signal::user_sigreturn,
};

#[allow(unused)]
extern "C" {
    fn stext();
    fn ssignal();
    fn esignal();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss();
    fn ebss();
    fn ekernel();
}

pub static KERNEL_SPACE: Once<MemorySet> = Once::new();

pub fn kernel_space_activate() {
    KERNEL_SPACE.get().unwrap().memory_activate();
}

#[inline(always)]
pub fn kernel_space_init() {
    KERNEL_SPACE.call_once(|| MemorySet::init_kernel_space());
    kernel_space_activate();
}

/// elf load result
pub struct ElfMemoryInfo {
    pub memory_set: MemorySet,
    pub entry_point: usize,
    pub user_sp: usize,
    pub auxs: Vec<AuxEntry>,
}

/// used in [`MemorySet::push_area`]
/// when mapping with data
pub struct MapAreaLoadDataInfo {
    pub start: usize,
    pub len: usize,
    pub offset: usize,
}

pub struct BrkAreaInfo {
    pub start: usize,
    pub end: usize,
    pub area: MapArea,
}

impl BrkAreaInfo {
    pub fn new_bare() -> Self {
        Self {
            start: 0,
            end: 0,
            area: MapArea::new_bare(),
        }
    }
}

pub struct MemorySet {
    /// page table tracks mapping info
    pub page_table: SyncUnsafeCell<PageTable>,

    /// map_areas tracks user data
    pub areas: Vec<MapArea>,

    /// stack
    pub stack: MapArea,

    /// brk
    pub brk: BrkAreaInfo,

    /// mmap manager
    pub mmap_manager: MmapManager,

    /// shm manager
    pub shm: ShmInfo,
}

impl MemorySet {
    /// create an new empty memory set without any allocation
    /// do not use this function directly, use [`Self::new_with_kernel`] instead
    fn new(page_table: PageTable) -> Self {
        Self {
            page_table: SyncUnsafeCell::new(page_table),
            areas: Vec::new(),
            stack: MapArea::new_bare(),
            brk: BrkAreaInfo::new_bare(),
            mmap_manager: MmapManager::new_bare(),
            shm: ShmInfo::new(),
        }
    }

    /// create a new memory set with root frame allocated
    pub fn new_allocated() -> Self {
        Self::new(PageTable::new_allocated())
    }

    /// create a new memory set with kernel space mapped,
    pub fn new_user_space() -> Self {
        let kernel_pt = KERNEL_SPACE.get().unwrap().page_table();
        let mut user_space = Self::new(PageTable::clone_from_other(&kernel_pt));
        user_space.map_sig_trampoline();
        user_space
    }

    #[inline(always)]
    pub fn page_table(&self) -> &mut PageTable {
        self.page_table.as_ref_mut()
    }

    pub fn root_ppn(&self) -> PhysPageNum {
        self.page_table().root_ppn()
    }

    /// switch into this memory set
    #[inline(always)]
    pub fn memory_activate(&self) {
        self.page_table().memory_activate();
    }

    /// push a map area into current memory set
    /// load data if provided
    pub async fn push_area(
        &mut self,
        mut map_area: MapArea,
        data_info: Option<MapAreaLoadDataInfo>,
    ) -> SysResult<()> {
        trace!(
            "push_area: [{:#X}, {:#X})",
            map_area.vpn_range().start().raw() << PAGE_WIDTH,
            map_area.vpn_range().end().raw() << PAGE_WIDTH
        );
        map_area.map_each(self.page_table());
        let pte = self
            .page_table()
            .find_pte(map_area.vpn_range().start())
            .unwrap();
        trace!(
            "create pte: ppn: {:#x}, flags: {:?}, raw_flag: {:?}",
            pte.ppn(),
            pte.flags(),
            pte.raw_flag(),
        );
        if let Some(data_info) = data_info {
            map_area.load_data(self.page_table(), data_info).await?;
        }
        self.areas.push(map_area); // bind life cycle
        Ok(())
    }

    /// create kernel space, used in [`KERNEL_SPACE`] initialization
    pub fn init_kernel_space() -> Self {
        let mut memory_set = MemorySet::new_allocated();
        #[cfg(target_arch = "riscv64")]
        {
            use arch::consts::{KERNEL_ADDR_OFFSET, KERNEL_VIRT_MEMORY_END};
            macro_rules! kernel_push_area {
                ($($start:expr, $end:expr, $permission:expr)*) => {
                    $(
                        crate::sched::utils::block_on(
                            memory_set.push_area(
                                MapArea::new(
                                    ($start as usize).into(),
                                    ($end as usize).into(),
                                    MapType::Direct,
                                    $permission,
                                    MapAreaType::KernelSpace,
                                    None,
                                ),
                                None,
                            )
                        ).expect("[init_kernel_space] push area error");
                    )*
                };
            }
            info!(
                "[kernel].text [{:#x}, {:#x})",
                stext as usize, etext as usize
            );
            info!(
                "[kernel].signal [{:#x}, {:#x}), entry: {:#x}",
                ssignal as usize, esignal as usize, user_sigreturn as usize
            );
            info!(
                "[kernel].rodata [{:#x}, {:#x})",
                srodata as usize, erodata as usize
            );
            info!(
                "[kernel].data [{:#x}, {:#x})",
                sdata as usize, edata as usize
            );
            info!("[kernel].bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
            info!(
                "[kernel] frame [{:#x}, {:#x})",
                ekernel as usize, KERNEL_VIRT_MEMORY_END as usize
            );
            kernel_push_area!(
                stext,   etext,   map_permission!(R, X)
                srodata, erodata, map_permission!(R)
                sdata,   edata,   map_permission!(R, W)
                sbss,    ebss,    map_permission!(R, W)
                ekernel, KERNEL_VIRT_MEMORY_END, map_permission!(R, W)
            );
            info!("mapping memory-mapped registers");
            for (start, len) in platform::MMIO_REGIONS {
                let s_addr = *start + KERNEL_ADDR_OFFSET;
                let e_addr = *start + *len + KERNEL_ADDR_OFFSET;
                debug!("[kernel] pushing MMIO area: [{:#x},{:#x})", s_addr, e_addr);
                kernel_push_area!(s_addr, e_addr, map_permission!(R, W));
            }
        }
        memory_set.page_table().mark_as_kernel();
        info!("[kernel] space initialized");
        memory_set
    }

    #[allow(unused)]
    pub fn load_dl_interp(&mut self, elf: &ElfFile) -> Option<usize> {
        let path = format!("{ROOT_NAME}/lib/libc.so");
        todo!("load_dl_interp")
    }

    pub async fn load_elf(elf_file: &Arc<dyn File>) -> SysResult<ElfMemoryInfo> {
        // read the beginning bytes to specify the header size
        let mut elf_mini_buf = [0u8; 64];
        elf_file.base_read(0, &mut elf_mini_buf).await?;
        let elf_error_handler = |x: &str| {
            error!("[load_elf] elf error: {:?}", x);
            Errno::ENOEXEC
        };
        let elf = ElfFile::new(&elf_mini_buf).map_err(elf_error_handler)?;

        // check: magic
        let magic = elf.header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");

        // get the real elf header
        let ph_entry_size = elf.header.pt2.ph_entry_size() as usize;
        let ph_offset = elf.header.pt2.ph_offset() as usize;
        let ph_count = elf.header.pt2.ph_count() as usize;
        let header_buf_len = ph_offset + ph_count * ph_entry_size;
        let mut elf_buf = vec![0u8; header_buf_len];
        elf_file.base_read(0, elf_buf.as_mut()).await?;
        let elf = ElfFile::new(elf_buf.as_slice()).map_err(elf_error_handler)?;

        // construct new memory set to hold elf data
        let mut memory_set = Self::new_user_space();
        let mut auxs: Vec<AuxEntry> = Vec::new(); // auxiliary vector
        let mut dl_flag = false; // dynamic link flag
        let mut entry_point = elf.header.pt2.entry_point() as usize;
        let mut head_va = 0;
        let mut end_vpn = None;

        for i in 0..ph_count {
            let ph = elf.program_header(i as u16).unwrap();
            use xmas_elf::program::Type::*;
            match ph.get_type().unwrap() {
                Load => {
                    let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                    let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                    if head_va == 0 {
                        head_va = start_va.raw();
                    }
                    let permission = map_permission!(U).merge_from_elf_flags(ph.flags());
                    let map_area = MapArea::new(
                        start_va,
                        end_va,
                        MapType::Framed,
                        permission,
                        MapAreaType::ElfBinary,
                        Some(Arc::clone(&elf_file)),
                        // start_va.offset(),
                    );
                    debug!(
                        "[map_elf] [{:#x}, {:#x}], permission: {:?}, ph offset {:#x}, file size {:#x}, mem size {:#x}",
                        start_va.raw(), end_va.raw(), permission,
                        ph.offset(),
                        ph.file_size(),
                        ph.mem_size()
                    );
                    end_vpn = Some(map_area.vpn_range.end());
                    memory_set
                        .push_area(
                            map_area,
                            Some(MapAreaLoadDataInfo {
                                start: ph.offset() as usize,
                                len: ph.file_size() as usize,
                                offset: start_va.offset(),
                            }),
                        )
                        .await?;
                }
                Interp => {
                    info!("elf Interp");
                    dl_flag = true;
                }
                _ => {}
            }
        }
        let end_va = VirtAddr::from(end_vpn.ok_or(Errno::ENOMEM)?);

        // user stack
        let user_stack_base = end_va + PAGE_SIZE; // stack bottom
        let user_stack_end = user_stack_base + USER_STACK_SIZE; // stack top
        let map_area = MapArea::new(
            user_stack_base,
            user_stack_base + USER_STACK_SIZE,
            MapType::Framed,
            map_permission!(U, R, W),
            MapAreaType::UserStack,
            None,
        );
        memory_set.stack = map_area;
        info!(
            "[memory_set] user stack mapped! [{:#x}, {:#x})",
            user_stack_base.raw(),
            user_stack_base.raw() + USER_STACK_SIZE
        );

        // user heap
        let user_heap_base = user_stack_end + PAGE_SIZE;
        memory_set.brk = BrkAreaInfo {
            start: user_heap_base.into(),
            end: user_heap_base.into(),
            area: MapArea::new(
                user_heap_base.into(),
                user_heap_base.into(),
                MapType::Framed,
                map_permission!(U, R, W),
                MapAreaType::UserHeap,
                None,
            ),
        };
        info!(
            "[memory_set] user heap inserted! [{:#x}, {:#x})",
            user_heap_base.raw(),
            user_heap_base.raw() + USER_HEAP_SIZE
        );

        // aux vector
        let ph_head_addr = head_va as u64 + elf.header.pt2.ph_offset() as u64;
        auxs.push(AuxEntry(AT_PHDR, ph_head_addr as usize));
        auxs.push(AuxEntry(AT_PHENT, elf.header.pt2.ph_entry_size() as usize)); // ELF64 header 64bytes
        auxs.push(AuxEntry(AT_PHNUM, ph_count as usize));
        auxs.push(AuxEntry(AT_PAGESZ, PAGE_SIZE as usize));
        if dl_flag {
            let interp_entry_point = memory_set.load_dl_interp(&elf);
            auxs.push(AuxEntry(AT_BASE, DL_INTERP_OFFSET));
            entry_point = interp_entry_point.unwrap();
        } else {
            auxs.push(AuxEntry(AT_BASE, 0));
        }
        auxs.push(AuxEntry(AT_FLAGS, 0));
        auxs.push(AuxEntry(AT_ENTRY, elf.header.pt2.entry_point() as usize));
        auxs.push(AuxEntry(AT_UID, 0));
        auxs.push(AuxEntry(AT_EUID, 0));
        auxs.push(AuxEntry(AT_GID, 0));
        auxs.push(AuxEntry(AT_EGID, 0));
        auxs.push(AuxEntry(AT_HWCAP, 0));
        auxs.push(AuxEntry(AT_CLKTCK, Arch::get_freq() as usize));
        auxs.push(AuxEntry(AT_SECURE, 0));

        let user_sp = user_stack_end.into();
        info!(
            "[load_elf] done, entry: {:#x}, sp: {:#x}",
            entry_point, user_sp
        );
        Ok(ElfMemoryInfo {
            memory_set,
            entry_point,
            user_sp, // stack grows downward, so return stack_end
            auxs,
        })
    }

    #[inline]
    pub async fn load_from_path(path: Path) -> SysResult<ElfMemoryInfo> {
        trace!("[load_elf] from path: {:?}", path);
        let elf_file = path.dentry().open()?;
        trace!("[load_elf] file name: {}", elf_file.name());
        Self::load_elf(&elf_file).await
    }

    /// clone current memory set,
    /// and mark the new memory set as copy-on-write
    /// used in sys_fork
    pub fn clone_cow(&mut self) -> (Self, usize) {
        trace!("[clone_cow] start");
        let mut new_set = Self::new_user_space();
        fn remap_cow(
            old_set: &MemorySet,
            vpn: VirtPageNum,
            new_set: &mut MemorySet,
            new_area: &mut MapArea,
            frame_tracker: &FrameTracker,
        ) {
            let old_pte = old_set.page_table().find_pte(vpn).unwrap();
            let old_flags = old_pte.flags();
            if old_flags.contains(MappingFlags::W) {
                let new_flags = flags_switch_to_cow(&old_flags);
                old_pte.set_flags(new_flags);
                new_set
                    .page_table()
                    .map(vpn, old_pte.ppn().into(), new_flags);
            } else {
                // fixme: mprotect could cause bugs here since we always share non-writable
                // memory between threads, maybe we should apply cow as well?
                new_set
                    .page_table()
                    .map(vpn, old_pte.ppn().into(), old_flags);
            }
            new_area.frame_map.insert(vpn, frame_tracker.clone());
        }

        // normal areas
        for area in self.areas.iter() {
            match area.area_type {
                MapAreaType::ElfBinary | MapAreaType::UserStack => {
                    let mut new_area = MapArea::from_another(area);
                    for vpn in area.vpn_range {
                        let frame_tracker = area.frame_map.get(&vpn).unwrap();
                        remap_cow(self, vpn, &mut new_set, &mut new_area, frame_tracker);
                    }
                    new_set.areas.push(new_area);
                }
                _ => {
                    warn!("[clone_cow] IGNORED area: {:?}", area.vpn_range);
                }
            }
        }

        // stack
        let area = &self.stack;
        let mut new_area = MapArea::from_another(area);
        for vpn in self.stack.vpn_range {
            if let Some(frame_tracker) = area.frame_map.get(&vpn) {
                remap_cow(self, vpn, &mut new_set, &mut new_area, frame_tracker);
            }
        }
        new_set.stack = new_area;

        // heap
        let area = &self.brk.area;
        let mut new_area = MapArea::from_another(area);
        for vpn in area.vpn_range {
            if let Some(frame_tracker) = area.frame_map.get(&vpn) {
                remap_cow(self, vpn, &mut new_set, &mut new_area, frame_tracker);
            }
        }
        new_set.brk.area = new_area;

        // mmap
        new_set.mmap_manager = self.mmap_manager.clone();
        for (vpn, frame_tracker) in self.mmap_manager.frame_trackers.iter() {
            let vpn = vpn.clone();
            debug!("[clone_cow] mmap vpn {:#x} is mapped as cow", vpn.raw());
            let old_pte = self.page_table().find_pte(vpn).unwrap();
            let old_flags = old_pte.flags();
            if old_flags.contains(MappingFlags::W) {
                let new_flags = flags_switch_to_cow(&old_flags);
                old_pte.set_flags(new_flags);
                new_set
                    .page_table()
                    .map(vpn, old_pte.ppn().into(), new_flags);
            } else {
                // fixme: mprotect could cause bugs as well
                new_set
                    .page_table()
                    .map(vpn, old_pte.ppn().into(), old_flags);
            }
            new_set
                .mmap_manager
                .frame_trackers
                .insert(vpn, frame_tracker.clone());
        }
        debug!(
            "[clone_cow] mmap_start: {:#x}, mmap_top: {:#x}",
            new_set.mmap_manager.mmap_start.raw(),
            new_set.mmap_manager.mmap_top.raw(),
        );

        // shm
        for shm_area in self.shm.shm_areas.iter() {
            let mut new_area = MapArea::from_another(shm_area);
            debug!(
                "[clone_cow] shm area: {:?} is mapped as cow",
                shm_area.vpn_range
            );
            for vpn in shm_area.vpn_range {
                if let Some(frame_tracker) = shm_area.frame_map.get(&vpn) {
                    remap_cow(self, vpn, &mut new_set, &mut new_area, frame_tracker);
                }
            }
            new_set.shm.shm_areas.push(new_area);
        }
        new_set.shm.shm_top = self.shm.shm_top;
        for (va, shm_tracker) in self.shm.shm_trackers.iter() {
            let new_shm_tracker = ShmTracker::new(shm_tracker.key);
            new_set.shm.shm_trackers.insert(*va, new_shm_tracker);
        }

        let root_ppn = new_set.root_ppn();
        (new_set, root_ppn.raw())
    }

    pub fn map_sig_trampoline(&mut self) {
        let sig_vpn = VirtAddr::from(SIG_TRAMPOLINE).floor();
        let sig_ppn = VirtAddr::from(user_sigreturn as usize)
            .floor()
            .kernel_translate_into_ppn();
        self.page_table()
            .map(sig_vpn.into(), sig_ppn.into(), pte_flags!(R, X, U));
    }

    pub fn lazy_alloc_stack(&mut self, vpn: VirtPageNum) {
        self.stack.map_one(vpn, self.page_table.as_ref_mut());
        Arch::tlb_flush();
    }

    pub fn lazy_alloc_brk(&mut self, vpn: VirtPageNum) {
        self.brk.area.map_one(vpn, self.page_table.as_ref_mut());
        Arch::tlb_flush();
    }

    pub fn brk_grow(&mut self, new_end_vpn: VirtPageNum) {
        self.brk
            .area
            .change_end_vpn(new_end_vpn, self.page_table.as_ref_mut());
        Arch::tlb_flush();
    }

    pub fn realloc_cow(&mut self, vpn: VirtPageNum, pte: &PageTableEntry) -> SysResult<()> {
        let old_ppn = PhysPageNum::from(pte.ppn());
        let old_flags = pte.flags();
        let new_flags = flags_switch_to_rw(&old_flags);
        if frame_refcount(old_ppn) == 1 {
            debug!("[realloc_cow] refcount is 1, set flags to RW: {new_flags:?}");
            self.page_table().set_flags(vpn, new_flags);
        } else {
            let frame = frame_alloc();
            let new_ppn = frame.ppn();
            let mut target = None;
            for area in self.areas.iter_mut() {
                if area.vpn_range.is_in_range(vpn) {
                    target = Some(area);
                    break;
                }
            }
            match target {
                Some(area) => {
                    area.frame_map.insert(vpn, frame);
                }
                None => {
                    if self.stack.vpn_range.is_in_range(vpn) {
                        self.stack.frame_map.insert(vpn, frame);
                    } else if self.brk.area.vpn_range.is_in_range(vpn) {
                        self.brk.area.frame_map.insert(vpn, frame);
                    } else if self.mmap_manager.is_in_space(vpn) {
                        self.mmap_manager.frame_trackers.insert(vpn, frame);
                    } else {
                        error!("[realloc_cow] vpn {:x?} is not in any area!!!", vpn);
                        return Err(Errno::ENOMEM);
                    }
                }
            }
            self.page_table()
                .remap_cow(vpn, new_ppn, old_ppn, new_flags);
            debug!(
                "[realloc_cow] done, refcount: old: [{:#x}: {:#x}], new: [{:#x}: {:#x}], flag: {:?}",
                old_ppn.raw(),
                frame_refcount(old_ppn),
                new_ppn.raw(),
                frame_refcount(new_ppn),
                new_flags,
            );
        }
        Arch::tlb_flush();
        Ok(())
    }

    pub fn attach_shm(&mut self, key: usize, start_va: VirtAddr) {
        let (start_pa, size) = shm_get_address_and_size(key);
        // println!("attach_shm start_pa {:#x}", start_pa.0);
        // println!("attach_shm start_va {:#x}", start_va.0);
        let flags = pte_flags!(V, U, W, R);
        let mut offset = 0;

        while offset < size {
            let va: VirtAddr = (start_va.raw() + offset).into();
            let pa: PhysAddr = (start_pa.raw() + offset).into();
            // println!("attach map va:{:x?} to pa{:x?}",va,pa);
            self.page_table().map(va.into(), pa.into(), flags);
            offset += PAGE_SIZE;
        }
        self.shm.shm_top = self.shm.shm_top.max(start_va.raw() + size);
        let shm_tracker = ShmTracker::new(key);

        self.shm.shm_trackers.insert(start_va, shm_tracker);
        let vma = MapArea::new(
            start_va,
            (start_va.raw() + size).into(),
            MapType::Framed,
            map_permission!(R, W),
            MapAreaType::Shared,
            None,
        );
        self.shm.shm_areas.push(vma);
    }

    pub fn detach_shm(&mut self, start_va: VirtAddr) -> usize {
        // println!("detach start_va:{:?}",start_va);
        let key = self.shm.shm_trackers.get(&start_va).unwrap().key;
        let (_, size) = shm_get_address_and_size(key);
        // println!("detach size:{:?}",size);
        let mut offset = 0;
        while offset < size {
            let va: VirtAddr = (start_va.raw() + offset).into();
            // println!("detach va:{:?}",va);
            unsafe { &mut (*self.page_table.get()) }.unmap(va.into());
            offset += PAGE_SIZE
        }
        self.shm.shm_trackers.remove(&start_va);
        let vpn: VirtPageNum = start_va.into();
        self.shm.shm_areas.retain(|x| x.vpn_range.start() != vpn);
        shm_get_nattch(key)
    }
}

pub async fn lazy_alloc_mmap<'a>(
    memory_set: &Arc<SpinLock<MemorySet>>,
    vpn: VirtPageNum,
) -> SysResult<()> {
    let mut ms = memory_set.lock();
    if !ms.mmap_manager.frame_trackers.contains_key(&vpn) {
        let frame = frame_alloc();
        let ppn = frame.ppn();
        let kvpn = frame.kernel_vpn();
        ms.mmap_manager.frame_trackers.insert(vpn, frame);
        let mut mmap_page = ms.mmap_manager.mmap_map.get(&vpn).cloned().unwrap();
        drop(ms);
        mmap_page.lazy_map_page(kvpn).await?;
        let ms = memory_set.lock();
        let pte_flags: MappingFlags = MappingFlags::from(mmap_page.prot) | MappingFlags::U;
        ms.page_table().map(vpn, ppn, pte_flags);
    } else {
        // todo: use suspend
        warn!("[mm] lazy_alloc_mmap: page already mapped, yield for it");
        while PageTable::from_ppn(Arch::current_root_ppn())
            .find_pte(vpn)
            .is_none()
        {
            yield_now().await;
        }
    }
    Arch::tlb_flush();
    Ok(())
}
