use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};

use arch::{Arch, ArchMemory, Exception};
use ksync::{cell::SyncUnsafeCell, mutex::SpinLock};
use lazy_static::lazy_static;

use super::{
    address::PhysAddr,
    frame::{frame_alloc, frame_refcount, FrameTracker},
    map_area::MapArea,
    mmap_manager::MmapManager,
    page_table::PageTable,
    pte::PageTableEntry,
};
use crate::{
    config::mm::{
        KERNEL_ADDR_OFFSET, KERNEL_VIRT_MEMORY_END, MMIO, PAGE_SIZE, PAGE_WIDTH, USER_HEAP_SIZE,
        USER_STACK_SIZE,
    },
    constant::time::CLOCK_FREQ,
    fs::{path::Path, vfs::basic::file::File},
    include::{process::auxv::*, result::Errno},
    map_permission,
    mm::{
        address::{VirtAddr, VirtPageNum},
        map_area::MapAreaType,
        permission::MapType,
    },
    syscall::SysResult,
};

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss();
    fn ebss();
    fn ekernel();
}

lazy_static! {
    pub static ref KERNEL_SPACE: SpinLock<MemorySet> =
        SpinLock::new(MemorySet::init_kernel_space());
}

/// lazily initialized kernel space token
/// please assure it's initialized before any user space token
pub static KERNEL_SPACE_TOKEN: AtomicUsize = AtomicUsize::new(0);

pub unsafe fn kernel_space_activate() {
    Arch::update_pagetable(KERNEL_SPACE_TOKEN.load(Ordering::Relaxed));
    Arch::tlb_flush();
}

/// elf load result
pub struct ElfMemoryInfo {
    pub memory_set: MemorySet,
    pub elf_entry: usize,
    pub user_sp: usize,
    pub auxs: Vec<AuxEntry>,
}

pub struct MemorySet {
    /// page table tracks mapping info
    pub page_table: SyncUnsafeCell<PageTable>,

    /// map_areas tracks user data
    pub areas: Vec<MapArea>,
    /// user stack area, lazily allocated
    pub user_stack_area: MapArea,
    /// user heap area, lazily allocated
    pub user_brk_area: MapArea,

    /// user stack base address
    pub user_stack_base: usize,
    /// user heap base address
    pub user_brk_start: usize,
    /// user heap end address
    pub user_brk: usize,

    /// mmap manager
    pub mmap_manager: MmapManager,
}

impl MemorySet {
    /// create an new empty memory set without any allocation
    /// do not use this function directly, use [`new_with_kernel`] instead
    ///
    /// use [`PageTable::new_bare`] to create a completly empty page table,
    /// or use [`PageTable::new_allocated`] to create one with root allocated
    pub fn new_bare(page_table: PageTable) -> Self {
        Self {
            page_table: SyncUnsafeCell::new(page_table),
            areas: Vec::new(),
            user_stack_area: MapArea::new_bare(),
            user_brk_area: MapArea::new_bare(),
            user_stack_base: 0,
            user_brk_start: 0,
            user_brk: 0,
            mmap_manager: MmapManager::new_bare(),
        }
    }

    #[inline(always)]
    pub fn page_table(&self) -> &mut PageTable {
        unsafe { &mut (*self.page_table.get()) }
    }

    /// get token, which will be written into satp
    pub fn token(&self) -> usize {
        self.page_table().token()
    }

    /// switch into this memory set
    #[inline(always)]
    pub unsafe fn activate(&self) {
        unsafe {
            self.page_table().activate();
        }
    }

    /// translate va into pa
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.page_table().translate_va(va)
    }

    /// push a map area into current memory set
    /// load data if provided
    pub fn push_area(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        info!(
            "push_area: [{:#X}, {:#X})",
            map_area.vpn_range().start().0 << PAGE_WIDTH,
            map_area.vpn_range().end().0 << PAGE_WIDTH
        );
        map_area.map_each(self.page_table());
        if let Some(data) = data {
            map_area.load_data(self.page_table(), data);
        }
        self.areas.push(map_area); // bind life cycle
    }

    /// create kernel space, used in [`KERNEL_SPACE`] initialization
    pub fn init_kernel_space() -> Self {
        let mut memory_set = MemorySet::new_bare(PageTable::new_allocated());
        macro_rules! kernel_push_area {
            ($($start:expr, $end:expr, $permission:expr)*) => {
                $(
                    memory_set.push_area(
                        MapArea::new(
                            ($start as usize).into(),
                            ($end as usize).into(),
                            MapType::Direct,
                            $permission,
                            MapAreaType::KernelSpace,
                        ),
                        None
                    );
                )*
            };
        }
        kernel_push_area!(
            stext,   etext,   map_permission!(R, X)
            srodata, erodata, map_permission!(R)
            sdata,   edata,   map_permission!(R, W)
            sbss,    ebss,    map_permission!(R, W)
            ekernel, KERNEL_VIRT_MEMORY_END, map_permission!(R, W)
        );
        info!("mapping memory-mapped registers");
        for (start, len) in MMIO {
            kernel_push_area!(
                *start + KERNEL_ADDR_OFFSET,
                *start + *len + KERNEL_ADDR_OFFSET,
                map_permission!(R, W)
            );
        }
        // trace!("[memory_set] sp: {:#x}", crate::arch::regs::get_sp());
        info!("[kernel] space initialized");
        info!(
            "[kernel].text [{:#x}, {:#x})",
            stext as usize, etext as usize
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
        KERNEL_SPACE_TOKEN.store(memory_set.token(), Ordering::SeqCst);
        memory_set
    }

    /// create a new memory set with kernel space mapped,
    pub fn new_with_kernel() -> Self {
        let mut memory_set = Self::new_bare(PageTable::new_bare());
        memory_set.page_table = SyncUnsafeCell::new(PageTable::clone_from_other(
            KERNEL_SPACE.lock().page_table(),
        ));
        memory_set
    }

    // TODO: is lazy allocation necessary? currently we don't use lazy alloc
    /// map user_stack_area
    pub fn map_user_stack(&mut self, start: usize, end: usize) {
        self.user_stack_base = start;
        let mut map_area = MapArea::new(
            start.into(),
            end.into(),
            MapType::Framed,
            map_permission!(U, R, W),
            MapAreaType::UserStack,
        );
        // map_area.map_each(self.page_table());
        self.user_stack_area = map_area;
    }

    /// map user_heap_area lazily
    pub fn map_user_heap(&mut self, start: usize, end: usize) {
        self.user_brk_start = start;
        self.user_brk = end;
        let map_area = MapArea::new(
            start.into(),
            end.into(),
            MapType::Framed,
            map_permission!(U, R, W),
            MapAreaType::UserHeap,
        );
        self.user_brk_area = map_area;
    }

    pub fn load_dl_interp(&mut self, elf: &Arc<dyn File>) -> Option<usize> {
        todo!("load_dl_interp")
    }

    /// load data from elf file
    pub async fn load_from_elf(elf_file: Arc<dyn File>) -> ElfMemoryInfo {
        let mut memory_set = Self::new_with_kernel();
        let mut auxs: Vec<AuxEntry> = Vec::new(); // auxiliary vector
        let mut dl_flag = false; // dynamic link flag

        // ! fixme: temp used for read all elf file
        let file_data = elf_file.read_all().await.unwrap();
        let elf = xmas_elf::ElfFile::new(file_data.as_slice()).unwrap();

        // check: magic
        let magic = elf.header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf.header.pt2.ph_count();
        let mut start_vpn = None;
        let mut end_vpn = None;

        // map pages by loaded program header
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            match ph.get_type().unwrap() {
                xmas_elf::program::Type::Load => {
                    let map_area = MapArea::new(
                        (ph.virtual_addr() as usize).into(),
                        ((ph.virtual_addr() + ph.mem_size()) as usize).into(),
                        MapType::Framed,
                        map_permission!(U).merge_from_elf_flags(ph.flags()),
                        MapAreaType::ElfBinary,
                    );
                    if start_vpn.is_none() {
                        start_vpn = Some(map_area.vpn_range.start());
                    }
                    end_vpn = Some(map_area.vpn_range.end());
                    memory_set.push_area(
                        map_area,
                        Some(
                            &elf.input
                                [ph.offset() as usize..(ph.offset() + ph.file_size()) as usize],
                        ),
                    );
                }
                xmas_elf::program::Type::Interp => {
                    dl_flag = true;
                }
                _ => {}
            }
        }
        let end_va: VirtAddr = end_vpn.unwrap().into();
        let elf_entry = elf.header.pt2.entry_point() as usize;
        info!("[load_elf] raw_entry: {:#x}", elf_entry);

        // user stack
        let user_stack_base: usize = usize::from(end_va) + PAGE_SIZE; // stack bottom
        let user_stack_end = user_stack_base + USER_STACK_SIZE; // stack top
        memory_set.map_user_stack(user_stack_base, user_stack_end);
        info!(
            "[memory_set] user stack mapped! [{:#x}, {:#x})",
            user_stack_base, user_stack_end
        );

        // user heap
        let user_heap_base: usize = user_stack_end + PAGE_SIZE;
        let user_heap_end: usize = user_heap_base;
        memory_set.map_user_heap(user_heap_base, user_heap_end);
        info!(
            "[memory_set] user heap mapped! [{:#x}, {:#x})",
            user_heap_base, user_heap_end
        );

        // aux vector
        let ph_head_addr = elf.header.pt2.ph_offset() as u64;
        auxs.push(AuxEntry(AT_PHDR, ph_head_addr as usize));
        auxs.push(AuxEntry(AT_PHENT, elf.header.pt2.ph_entry_size() as usize)); // ELF64 header 64bytes
        auxs.push(AuxEntry(AT_PHNUM, ph_count as usize));
        auxs.push(AuxEntry(AT_PAGESZ, PAGE_SIZE as usize));
        if dl_flag {
            // let interp_entry_point = memory_set.load_dl_interp(&elf).await;
            // auxv.push(AuxEntry(AT_BASE, DL_INTERP_OFFSET));
            // elf_entry = interp_entry_point.unwrap();
            unimplemented!()
        } else {
            auxs.push(AuxEntry(AT_BASE, 0));
        }
        auxs.push(AuxEntry(AT_FLAGS, 0 as usize));
        auxs.push(AuxEntry(AT_ENTRY, elf.header.pt2.entry_point() as usize));
        auxs.push(AuxEntry(AT_UID, 0 as usize));
        auxs.push(AuxEntry(AT_EUID, 0 as usize));
        auxs.push(AuxEntry(AT_GID, 0 as usize));
        auxs.push(AuxEntry(AT_EGID, 0 as usize));
        auxs.push(AuxEntry(AT_HWCAP, 0 as usize));
        auxs.push(AuxEntry(AT_CLKTCK, CLOCK_FREQ as usize));
        auxs.push(AuxEntry(AT_SECURE, 0 as usize));

        ElfMemoryInfo {
            memory_set,
            elf_entry,
            user_sp: user_stack_end - 16, // stack grows downward, so return stack_end
            auxs,
        }
    }

    pub async fn load_from_path(path: Path) -> ElfMemoryInfo {
        info!("[load_elf] from path: {:?}", path);
        let elf_file = path.dentry().open().unwrap();
        info!("[load_elf] file name: {}", elf_file.name());
        MemorySet::load_from_elf(elf_file).await
    }

    /// clone current memory set,
    /// and mark the new memory set as copy-on-write
    /// used in sys_fork
    pub fn clone_cow(&mut self) -> Self {
        trace!("[clone_cow] start");
        let mut new_set = Self::new_with_kernel();
        let remap_cow = |vpn: VirtPageNum,
                         new_set: &mut MemorySet,
                         new_area: &mut MapArea,
                         frame_tracker: &FrameTracker| {
            let old_pte = self.page_table().translate_vpn(vpn).unwrap();
            let old_flags = old_pte.flags();
            if !old_flags.is_writable() {
                new_set.page_table().map(vpn, old_pte.ppn(), old_flags);
                new_area.frame_map.insert(vpn, frame_tracker.clone());
            } else {
                let new_flags = old_flags.switch_to_cow();
                self.page_table().set_flags(vpn, new_flags);
                new_set.page_table().map(vpn, old_pte.ppn(), new_flags);
                new_area.frame_map.insert(vpn, frame_tracker.clone());
                trace!("remap_cow: vpn = {:#x}, new_flags = {:?}", vpn.0, new_flags);
            }
        };

        // normal areas
        for area in self.areas.iter() {
            assert!(area.area_type == MapAreaType::ElfBinary);
            let mut new_area = MapArea::from_another(area);
            for vpn in area.vpn_range {
                // no `let Some(...)` since we always alloc it
                let frame_tracker = area.frame_map.get(&vpn).unwrap();
                remap_cow(vpn, &mut new_set, &mut new_area, frame_tracker);
            }
            new_set.areas.push(new_area);
        }

        // stack
        trace!(
            "mapping stack as cow, range: [{:#x}, {:#x})",
            self.user_stack_base,
            self.user_stack_base + USER_STACK_SIZE,
        );
        let area = &self.user_stack_area;
        let mut new_area = MapArea::from_another(&self.user_stack_area);
        for vpn in self.user_stack_area.vpn_range {
            if let Some(frame_tracker) = area.frame_map.get(&vpn) {
                remap_cow(vpn, &mut new_set, &mut new_area, frame_tracker);
            }
        }
        new_set.user_stack_area = new_area;

        // heap
        trace!(
            "mapping heap as cow, range: [{:#x}, {:#x})",
            self.user_brk_start,
            self.user_brk_start + USER_HEAP_SIZE,
        );
        let area = &self.user_brk_area;
        let mut new_area = MapArea::from_another(area);
        for vpn in area.vpn_range {
            trace!(
                "[clone_cow] vpn: {:#x}, range: [{:#x}, {:#x})",
                vpn.0,
                area.vpn_range.start().0,
                area.vpn_range.end().0
            );
            for it in area.frame_map.iter() {
                trace!("[clone_cow] frame_map: {:#x}", it.0 .0);
            }
            if let Some(frame_tracker) = area.frame_map.get(&vpn) {
                remap_cow(vpn, &mut new_set, &mut new_area, frame_tracker);
            }
        }
        new_set.user_brk_area = new_area;

        new_set
    }

    pub fn lazy_alloc_stack(&mut self, vpn: VirtPageNum) {
        self.user_stack_area
            .map_one(vpn, unsafe { &mut (*self.page_table.get()) });
        Arch::tlb_flush();
    }

    pub fn lazy_alloc_brk(&mut self, vpn: VirtPageNum) {
        self.user_brk_area
            .map_one(vpn, unsafe { &mut (*self.page_table.get()) });
        Arch::tlb_flush();
    }

    pub fn brk_grow(&mut self, new_brk_vpn: VirtPageNum) {
        self.user_brk_area
            .change_end_vpn(new_brk_vpn, unsafe { &mut (*self.page_table.get()) });
        // tlb is already flushed in `modify_end`
    }

    pub fn realloc_cow(&mut self, vpn: VirtPageNum, pte: PageTableEntry) {
        let old_ppn = pte.ppn();
        let old_flags = pte.flags();
        let new_flags = old_flags.switch_to_rw();
        if frame_refcount(old_ppn) == 1 {
            trace!("refcount == 1, set flags to RW");
            self.page_table().set_flags(vpn, new_flags);
        } else {
            let frame = frame_alloc();
            let new_ppn = frame.ppn();
            let mut flag = false;
            for area in self.areas.iter_mut() {
                if area.vpn_range.is_in_range(vpn) {
                    area.frame_map.insert(vpn, frame.clone());
                    flag = true;
                    break;
                }
            }
            if !flag {
                if self.user_stack_area.vpn_range.is_in_range(vpn) {
                    self.user_stack_area.frame_map.insert(vpn, frame.clone());
                } else if self.user_brk_area.vpn_range.is_in_range(vpn) {
                    self.user_brk_area.frame_map.insert(vpn, frame.clone());
                } else {
                    panic!("[realloc_cow] vpn is not in any area!!!");
                }
            }
            self.page_table()
                .remap_cow(vpn, new_ppn, old_ppn, new_flags);
            Arch::tlb_flush();
            trace!(
                "[realloc_cow] done!!! refcount: old: [{:#x}: {:#x}], new: [{:#x}: {:#x}]",
                old_ppn.0,
                frame_refcount(old_ppn),
                new_ppn.0,
                frame_refcount(new_ppn),
            );
        }
    }

    // TODO: add mmap check
    /// # memory validate
    /// Check if is the copy-on-write/lazy-alloc pages triggered the page fault.
    ///
    /// As for cow, clone pages for the writer(aka current task),
    /// but should keep original page as cow since it might still be shared.
    /// Note that if the reference count is one, there's no need to clone pages.
    ///
    /// As for lazy alloc, realloc pages for the task.
    /// Associated pages: stack, heap, mmap
    ///
    /// Return value: true if successfully handled lazy alloc or copy-on-write;
    ///               false if the page fault is not in any alloc area.
    ///
    /// usages: when any kernel allocation in user_space happens, call this fn;
    /// when user pagefault happens, call this func to check allocation.
    pub async fn validate(
        &mut self,
        vpn: VirtPageNum,
        exception: Option<Exception>,
        pte: Option<PageTableEntry>,
    ) -> SysResult<()> {
        if let Some(pte) = pte {
            let flags = pte.flags();
            if flags.is_cow() {
                trace!(
                    "[memory_validate] realloc COW at vpn: {:#x}, pte: {:#x}, flags: {:?}",
                    vpn.0,
                    pte.0,
                    pte.flags()
                );
                self.realloc_cow(vpn, pte);
                Ok(())
            } else if exception.is_some() && exception.unwrap() == Exception::StorePageFault {
                error!(
                    "[memory_validate] store at invalid area, flags: {:?}",
                    flags
                );
                Err(Errno::EFAULT)
            } else {
                error!("unknown error in memory validate");
                Ok(())
            }
        } else {
            if self.user_stack_area.vpn_range.is_in_range(vpn) {
                info!("[memory_validate] realloc stack");
                self.lazy_alloc_stack(vpn);
                Ok(())
            } else if self.user_brk_area.vpn_range.is_in_range(vpn) {
                info!("[memory_validate] realloc heap");
                self.lazy_alloc_brk(vpn);
                Ok(())
            } else if self.mmap_manager.is_in_space(vpn) {
                info!("[memory_validate] realloc mmap");
                let res = self.lazy_alloc_mmap(vpn).await;
                if let Err(res) = res {
                    warn!("[memory_validate] error when realloc mmap: {}", res);
                }
                Ok(())
            } else {
                error!("[memory_validate] not in any alloc area");
                Err(Errno::EFAULT)
            }
        }
    }
}

#[allow(unused)]
pub fn remap_test() {
    debug!("remap_test");

    let mut kernel_space = KERNEL_SPACE.lock();

    let mid_text: VirtAddr = (stext as usize / 2 + etext as usize / 2).into();
    let mid_rodata: VirtAddr = (srodata as usize / 2 + erodata as usize / 2).into();
    let mid_data: VirtAddr = (sdata as usize / 2 + edata as usize / 2).into();
    let mid_bss: VirtAddr = (sbss as usize / 2 + ebss as usize / 2).into();
    let mid_frame: VirtAddr = (ekernel as usize / 2 + KERNEL_VIRT_MEMORY_END as usize / 2).into();

    debug!(
        "mid_text: {:#x} => {:#x}",
        mid_text.0,
        kernel_space.translate_va(mid_text).unwrap().0
    );
    debug!(
        "mid_rodata: {:#x} => {:#x}",
        mid_rodata.0,
        kernel_space.translate_va(mid_rodata).unwrap().0
    );
    debug!(
        "mid_data: {:#x} => {:#x}",
        mid_data.0,
        kernel_space.translate_va(mid_data).unwrap().0
    );
    debug!(
        "mid_bss: {:#x} => {:#x}",
        mid_bss.0,
        kernel_space.translate_va(mid_bss).unwrap().0
    );
    debug!(
        "mid_frame: {:#x} => {:#x}",
        mid_frame.0,
        kernel_space.translate_va(mid_frame).unwrap().0
    );

    debug!("remap_test end");
}
