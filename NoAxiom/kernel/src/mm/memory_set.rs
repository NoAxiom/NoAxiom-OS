use alloc::{string::String, sync::Arc, vec::Vec};

use arch::{Arch, ArchInt, ArchMemory, ArchPageTableEntry, ArchTime, MappingFlags, PageTableEntry};
use config::mm::{DL_INTERP_OFFSET, SIG_TRAMPOLINE, USER_HEAP_SIZE};
use include::errno::Errno;
use ksync::cell::SyncUnsafeCell;
use memory::frame::can_frame_alloc_loosely;
use spin::Once;
use xmas_elf::ElfFile;

use super::{
    address::{PhysAddr, PhysPageNum},
    frame::{frame_alloc, frame_refcount, FrameTracker},
    map_area::MapArea,
    mmap_manager::MmapManager,
    page_table::{flags_switch_to_rw, PageTable},
    shm::{ShmInfo, ShmTracker},
};
use crate::{
    config::mm::{PAGE_SIZE, PAGE_WIDTH, USER_STACK_SIZE},
    cpu::current_task,
    fs::{path::Path, vfs::basic::file::File},
    include::{mm::MmapFlags, process::auxv::*},
    map_permission,
    mm::{
        address::{VirtAddr, VirtPageNum},
        map_area::MapAreaType,
        page_table::flags_switch_to_cow,
        permission::MapType,
        shm::SHM_MANAGER,
    },
    pte_flags, return_errno,
    syscall::SysResult,
    task::signal::user_sigreturn,
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

pub struct RawElfInfo {
    head_va: VirtAddr,
    end_va: VirtAddr,
    ph_offset: usize,
    ph_count: usize,
    ph_entry_size: usize,
    entry_point: usize,
    dl_interp: Option<Path>,
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
pub struct MapAreaLoadDataInfo<'a> {
    pub start: usize,
    pub len: usize,
    pub offset: usize,
    pub slice: &'a [u8],
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

    /// switch into this memory set
    #[inline(always)]
    pub fn memory_activate(&self) {
        self.page_table().memory_activate();
    }

    /// push a map area into current memory set
    /// load data if provided
    pub fn push_area(
        &mut self,
        mut map_area: MapArea,
        data_info: Option<MapAreaLoadDataInfo<'_>>,
    ) -> SysResult<()> {
        trace!(
            "push_area: [{:#X}, {:#X})",
            map_area.vpn_range().start().raw() << PAGE_WIDTH,
            map_area.vpn_range().end().raw() << PAGE_WIDTH
        );
        map_area.map_each(self.page_table())?;
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
            map_area.load_data(self.page_table(), data_info);
        }
        self.areas.push(map_area); // bind life cycle
        Ok(())
    }

    /// create kernel space, used in [`KERNEL_SPACE`] initialization
    pub fn init_kernel_space() -> Self {
        #[allow(unused_mut)]
        let mut memory_set = MemorySet::new_allocated();
        #[cfg(target_arch = "riscv64")]
        {
            use arch::consts::{IO_ADDR_OFFSET, KERNEL_VIRT_MEMORY_END};
            use device::devconf::get_mmio_regions;
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
                            ).unwrap(),
                            None,
                        ).unwrap();
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
            let mmio_regions = get_mmio_regions();
            for (start, len) in mmio_regions {
                let s_addr = *start + IO_ADDR_OFFSET;
                let e_addr = *start + *len + IO_ADDR_OFFSET;
                debug!("[kernel] pushing MMIO area: [{:#x},{:#x})", s_addr, e_addr);
                memory_set
                    .push_area(
                        MapArea::new(
                            (s_addr as usize).into(),
                            (e_addr as usize).into(),
                            MapType::IOPort, // IO area
                            map_permission!(R, W),
                            MapAreaType::KernelSpace,
                        )
                        .unwrap(),
                        None,
                    )
                    .unwrap();
            }
        }
        memory_set.page_table().mark_as_kernel();
        info!("[kernel] space initialized");
        memory_set
    }

    pub async fn map_elf(
        self: &mut Self,
        elf_file: &Arc<dyn File>,
        base_offset: usize,
        is_dl_interp: bool,
    ) -> SysResult<RawElfInfo> {
        info!("[map_elf] start, dl_interp: {is_dl_interp}");

        // read the beginning bytes to specify the header size
        let handler = |msg| {
            error!("[load_elf] elf parse error: {}", msg);
            match is_dl_interp {
                true => Errno::ELIBBAD,
                false => Errno::ENOEXEC,
            }
        };
        let elf_buf = elf_file.read_all().await?;
        let elf = ElfFile::new(elf_buf.as_slice()).map_err(handler)?;

        // check: magic
        let magic = elf.header.pt1.magic;
        if magic != [0x7f, 0x45, 0x4c, 0x46] {
            handler("invalid magic");
        }

        // get the real elf header
        let ph_entry_size = elf.header.pt2.ph_entry_size() as usize;
        let ph_offset = elf.header.pt2.ph_offset() as usize;
        let ph_count = elf.header.pt2.ph_count() as usize;

        // construct new memory set to hold elf data
        let mut dl_interp = None;
        let mut head_va = None;
        let mut end_vpn = None;
        let mut frame_req_num = 0;
        let mut areas = Vec::new();

        for i in 0..ph_count {
            let ph = elf.program_header(i as u16).map_err(handler)?;
            use xmas_elf::program::Type::*;
            match ph.get_type().map_err(handler)? {
                Load => {
                    let start_va: VirtAddr = (ph.virtual_addr() as usize + base_offset).into();
                    let end_va: VirtAddr =
                        ((ph.virtual_addr() + ph.mem_size()) as usize + base_offset).into();
                    if head_va.is_none() {
                        head_va = Some(start_va);
                    }
                    let permission = map_permission!(U).merge_from_elf_flags(ph.flags());
                    let map_area = MapArea::new(
                        start_va,
                        end_va,
                        MapType::Framed,
                        permission,
                        MapAreaType::ElfBinary,
                    )?;
                    info!(
                        "[map_elf] [{:#x}, {:#x}], permission: {:?}, ph offset {:#x}, file size {:#x}, mem size {:#x}",
                        start_va.raw(), end_va.raw(), permission,
                        ph.offset(),
                        ph.file_size(),
                        ph.mem_size()
                    );
                    end_vpn = Some(map_area.vpn_range.end());
                    // we won't map the area immediately
                    // alloc it after all checks are done
                    frame_req_num += map_area.vpn_range.page_count();
                    areas.push((
                        map_area,
                        Some(MapAreaLoadDataInfo {
                            start: ph.offset() as usize,
                            len: ph.file_size() as usize,
                            offset: start_va.offset(),
                            slice: &elf_buf,
                        }),
                    ));
                }
                Interp => {
                    if is_dl_interp {
                        error!("[load_elf] detect recursive dl_interp, skip dl_interp loading");
                        return_errno!(Errno::ELIBBAD);
                    }
                    if dl_interp.is_some() {
                        error!("[load_elf] dl_interp already set");
                        return_errno!(Errno::ENOEXEC);
                    }
                    let mut buf = vec![0u8; ph.file_size() as usize];
                    if buf.ends_with(&[0u8; 1]) {
                        buf.pop();
                    }
                    elf_file
                        .read_at(ph.offset() as usize, buf.as_mut_slice())
                        .await?;
                    let path = format!(
                        "{}",
                        String::from_utf8(buf)
                            .map_err(|_| Errno::ENOEXEC)?
                            .trim_end_matches('\0')
                    );
                    // match path.as_str() {
                    //     // rv
                    //     "/lib/ld-linux-riscv64-lp64d.so.1" | "/lib/ld-linux-riscv64-lp64.so.1" =>
                    // {         path =
                    // format!("/glibc/lib/ld-linux-riscv64-lp64d.so.1");     }
                    //     "/lib/libc.so.6" => {
                    //         path = format!("/glibc/lib/libc.so");
                    //     }
                    //     "/lib/libm.so.6" => {
                    //         path = format!("/glibc/lib/libm.so");
                    //     }
                    //     "/lib/ld-musl-riscv64-sf.so.1" => {
                    //         path = format!("/musl/lib/libc.so");
                    //     }
                    //     // la
                    //     "/lib64/ld-linux-loongarch-lp64d.so.1" => {
                    //         path = format!("/glibc/lib/ld-linux-loongarch-lp64d.so.1");
                    //     }
                    //     "/lib64/libc.so.6" | "/usr/lib64/libc.so.6" => {
                    //         path = format!("/glibc/lib/libc.so.6");
                    //     }
                    //     "/lib64/libm.so.6" | "/usr/lib64/libm.so.6" => {
                    //         path = format!("/glibc/lib/libm.so.6");
                    //     }
                    //     "/lib/ld-musl-loongarch64-lp64d.so.1"
                    //     | "/lib64/ld-musl-loongarch-lp64d.so.1" => {
                    //         path = format!("/musl/lib/libc.so");
                    //     }
                    //     s => {
                    //         panic!(
                    //             "[load_dl_interp] unknown interpreter path: {s}, path = {}",
                    //             path
                    //         );
                    //     }
                    // }
                    info!("[load_elf] find interp path: {}", path);
                    assert!(Arch::is_external_interrupt_enabled());
                    dl_interp = Some(Path::from_string(path, current_task().unwrap()).unwrap());
                }
                _ => {}
            }
        }

        // reserve 20% more frames for later use
        trace!("frame_req_num: {}", frame_req_num);
        if !can_frame_alloc_loosely(frame_req_num) {
            return_errno!(Errno::ENOMEM, "no enough frames to load elf");
        }

        // fetch start and end va
        let head_va = head_va.ok_or(Errno::ENOMEM)?;
        let end_va = VirtAddr::from(end_vpn.ok_or(Errno::ENOMEM)?);
        let entry_point = elf.header.pt2.entry_point() as usize + base_offset;

        // checks are done! now push areas into memory set
        for (area, info) in areas {
            self.push_area(area, info)?;
        }

        Ok(RawElfInfo {
            head_va,
            end_va,
            ph_offset,
            ph_count,
            ph_entry_size,
            entry_point,
            dl_interp,
        })
    }

    pub async fn load_elf(file: &Arc<dyn File>) -> SysResult<ElfMemoryInfo> {
        let mut memory_set = MemorySet::new_user_space();
        let elf = memory_set.map_elf(file, 0, false).await?;

        // user stack
        let user_stack_base = elf.end_va + PAGE_SIZE; // stack bottom
        let user_stack_top = user_stack_base + USER_STACK_SIZE; // stack top
        let map_area = MapArea::new(
            user_stack_base,
            user_stack_base + USER_STACK_SIZE,
            MapType::Framed,
            map_permission!(U, R, W),
            MapAreaType::UserStack,
        )?;
        memory_set.stack = map_area;
        info!(
            "[memory_set] user stack mapped! [{:#x}, {:#x})",
            user_stack_base.raw(),
            user_stack_base.raw() + USER_STACK_SIZE
        );
        memory_set.lazy_alloc_stack(VirtPageNum::from(user_stack_top - PAGE_SIZE))?;

        // user heap
        let user_heap_base = user_stack_top + PAGE_SIZE;
        memory_set.brk = BrkAreaInfo {
            start: user_heap_base.into(),
            end: user_heap_base.into(),
            area: MapArea::new(
                user_heap_base.into(),
                user_heap_base.into(),
                MapType::Framed,
                map_permission!(U, R, W),
                MapAreaType::UserHeap,
            )?,
        };
        info!(
            "[memory_set] user heap inserted! [{:#x}, {:#x})",
            user_heap_base.raw(),
            user_heap_base.raw() + USER_HEAP_SIZE
        );

        // aux vector
        let mut auxs: Vec<AuxEntry> = Vec::new(); // auxiliary vector
        let mut entry_point = elf.entry_point;
        let ph_head_addr = elf.head_va.raw() as u64 + elf.ph_offset as u64;
        auxs.push(AuxEntry(AT_PHDR, ph_head_addr as usize));
        auxs.push(AuxEntry(AT_PHENT, elf.ph_entry_size as usize)); // ELF64 header 64bytes
        auxs.push(AuxEntry(AT_PHNUM, elf.ph_count as usize));
        auxs.push(AuxEntry(AT_PAGESZ, PAGE_SIZE as usize));
        if let Some(path) = elf.dl_interp {
            auxs.push(AuxEntry(AT_BASE, DL_INTERP_OFFSET));
            let dl_interp_file = path.dentry().open()?;
            let dl_interp_info = memory_set
                .map_elf(&dl_interp_file, DL_INTERP_OFFSET, true)
                .await?;
            entry_point = dl_interp_info.entry_point;
        } else {
            auxs.push(AuxEntry(AT_BASE, 0));
        }
        auxs.push(AuxEntry(AT_FLAGS, 0));
        auxs.push(AuxEntry(AT_ENTRY, elf.entry_point as usize)); // use old entry
        auxs.push(AuxEntry(AT_UID, 0));
        auxs.push(AuxEntry(AT_EUID, 0));
        auxs.push(AuxEntry(AT_GID, 0));
        auxs.push(AuxEntry(AT_EGID, 0));
        auxs.push(AuxEntry(AT_HWCAP, 0));
        auxs.push(AuxEntry(AT_CLKTCK, Arch::get_freq() as usize));
        auxs.push(AuxEntry(AT_SECURE, 0));

        let user_sp = user_stack_top.into();
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

    /// clone current memory set,
    /// and mark the new memory set as copy-on-write
    /// used in sys_fork
    pub fn clone_cow(&mut self) -> Self {
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
                new_set
                    .page_table()
                    .map(vpn, old_pte.ppn().into(), old_flags);
            }
            new_area.frame_map.insert(vpn, frame_tracker.clone());
        }

        // normal areas
        for area in self.areas.iter() {
            match area.area_type {
                MapAreaType::ElfBinary => {
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
        new_set.brk.start = self.brk.start;
        new_set.brk.end = self.brk.end;
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
            trace!("[clone_cow] mmap vpn {:#x} is mapped as cow", vpn.raw());
            let old_pte = self.page_table().find_pte(vpn).unwrap();
            let old_flags = old_pte.flags();
            if !self
                .mmap_manager
                .mmap_map
                .get(&vpn)
                .unwrap()
                .flags
                .contains(MmapFlags::MAP_SHARED)
            {
                if old_flags.contains(MappingFlags::W) {
                    let new_flags = flags_switch_to_cow(&old_flags);
                    old_pte.set_flags(new_flags);
                    new_set
                        .page_table()
                        .map(vpn, old_pte.ppn().into(), new_flags);
                } else {
                    new_set
                        .page_table()
                        .map(vpn, old_pte.ppn().into(), old_flags);
                }
            } else {
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

        // shm: map it directly since shared memory needn't extra alloc
        for shm_area in self.shm.shm_areas.iter() {
            // we save data slice in shm manager, so the frame map is empty
            assert!(shm_area.frame_map.is_empty());
            let new_area = MapArea::from_another(shm_area);
            info!(
                "[clone_cow] shm area: {:?} is mapped as cow",
                shm_area.vpn_range
            );
            for vpn in shm_area.vpn_range {
                let pte = self.page_table().find_pte(vpn).unwrap();
                let flag = pte.flags();
                new_set.page_table().map(vpn, pte.ppn().into(), flag);
                // trace!(
                //     "new_set: {:#x} flags: {:?}",
                //     new_set.page_table().find_pte(vpn).unwrap().ppn(),
                //     new_set.page_table().find_pte(vpn).unwrap().flags()
                // );
            }
            new_set.shm.shm_areas.push(new_area);
        }
        new_set.shm.shm_top = self.shm.shm_top;
        for (va, shm_tracker) in self.shm.shm_trackers.iter() {
            let new_shm_tracker = ShmTracker::new(shm_tracker.key);
            new_set.shm.shm_trackers.insert(*va, new_shm_tracker);
        }
        new_set
    }

    pub fn map_sig_trampoline(&mut self) {
        let sig_vpn = VirtAddr::from(SIG_TRAMPOLINE).floor();
        let sig_ppn = VirtAddr::from(user_sigreturn as usize)
            .floor()
            .kernel_translate_into_ppn();
        self.page_table()
            .map(sig_vpn.into(), sig_ppn.into(), pte_flags!(R, X, U));
    }

    pub fn lazy_alloc_stack(&mut self, vpn: VirtPageNum) -> SysResult<()> {
        self.stack.map_one(vpn, self.page_table.as_ref_mut())?;
        Arch::tlb_flush();
        Ok(())
    }

    pub fn lazy_alloc_brk(&mut self, vpn: VirtPageNum) -> SysResult<()> {
        self.brk.area.map_one(vpn, self.page_table.as_ref_mut())?;
        Arch::tlb_flush();
        Ok(())
    }

    pub fn brk_grow(&mut self, new_end_vpn: VirtPageNum) -> SysResult<()> {
        self.brk
            .area
            .change_end_vpn(new_end_vpn, self.page_table.as_ref_mut())?;
        Arch::tlb_flush();
        Ok(())
    }

    pub fn realloc_cow(&mut self, vpn: VirtPageNum, pte: &PageTableEntry) -> SysResult<()> {
        let old_ppn = PhysPageNum::from(pte.ppn());
        let old_flags = pte.flags();
        let new_flags = flags_switch_to_rw(&old_flags);
        if frame_refcount(old_ppn) == 1 {
            trace!("[realloc_cow] refcount is 1, set flags to RW: {new_flags:?}");
            self.page_table().set_flags(vpn, new_flags);
        } else {
            let frame = frame_alloc().unwrap();
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
            trace!(
                "[realloc_cow] done, old: {:#x}, new: {:#x}, flag: {:?}",
                old_ppn.raw(),
                new_ppn.raw(),
                new_flags,
            );
        }
        Arch::tlb_flush();
        Ok(())
    }

    pub fn attach_shm(&mut self, key: usize, start_va: VirtAddr) -> SysResult<()> {
        let (start_pa, size) = SHM_MANAGER.lock().get_address_and_size(key);
        warn!("attach_shm start_pa {:#x}", start_pa.raw());
        warn!("attach_shm start_va {:#x}", start_va.raw());
        let flags = pte_flags!(V, U, W, R);
        let mut offset = 0;

        while offset < size {
            let va = start_va + offset;
            let pa = PhysAddr::from(start_pa.raw() + offset);
            warn!("attach map va:{:x?} to pa{:x?}", va, pa);
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
        )?;
        self.shm.shm_areas.push(vma);
        Ok(())
    }

    pub fn detach_shm(&mut self, start_va: VirtAddr) -> usize {
        warn!("detach start_va:{:?}", start_va);
        let key = self.shm.shm_trackers.get(&start_va).unwrap().key;
        let (_, size) = SHM_MANAGER.lock().get_address_and_size(key);
        warn!("detach size:{:?}", size);
        let mut offset = 0;
        while offset < size {
            let va = start_va + offset;
            warn!("detach va:{:?}", va);
            self.page_table().unmap(va.into());
            offset += PAGE_SIZE
        }
        self.shm.shm_trackers.remove(&start_va);
        let vpn: VirtPageNum = start_va.into();
        self.shm.shm_areas.retain(|x| x.vpn_range.start() != vpn);
        SHM_MANAGER.lock().get_nattch(key)
    }
}
