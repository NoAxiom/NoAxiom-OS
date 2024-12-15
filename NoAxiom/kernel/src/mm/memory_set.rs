use alloc::{sync::Arc, vec::Vec};
use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use lazy_static::lazy_static;
use riscv::register::satp;

use super::{address::PhysAddr, map_area::MapArea, page_table::PageTable};
use crate::{
    config::mm::{
        KERNEL_ADDR_OFFSET, KERNEL_VIRT_MEMORY_END, MMIO, PAGE_SIZE, PAGE_WIDTH, USER_HEAP_SIZE,
        USER_STACK_SIZE,
    },
    fs::File,
    map_permission,
    mm::{
        address::{VirtAddr, VirtPageNum},
        map_area::MapAreaType,
        permission::MapType,
    },
    sync::mutex::SpinMutex,
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
    pub static ref KERNEL_SPACE: SpinMutex<MemorySet> =
        SpinMutex::new(MemorySet::init_kernel_space());
}

/// lazily initialized kernel space token
/// please assure it's initialized before any user space token
pub static KERNEL_SPACE_TOKEN: AtomicUsize = AtomicUsize::new(0);

pub unsafe fn kernel_space_activate() {
    unsafe {
        satp::write(KERNEL_SPACE_TOKEN.load(Ordering::Relaxed));
        asm!("sfence.vma");
    }
}

/// elf load result
pub struct ElfMemoryInfo {
    pub memory_set: MemorySet,
    pub elf_entry: usize,
    pub user_sp: usize,
}

pub struct MemorySet {
    /// page table tracks mapping info
    pub page_table: PageTable,

    /// map_areas tracks user data
    pub areas: Vec<MapArea>,

    /// user stack area, lazily allocated
    pub user_stack_area: Option<MapArea>,

    /// user heap area, lazily allocated
    pub user_heap_area: Option<MapArea>,

    /// user stack base address
    pub user_stack_base: usize,

    /// user heap base address, aka brk
    pub user_heap_base: usize,
}

impl MemorySet {
    /// create an new empty memory set without any allocation
    /// do not use this function directly, use [`new_with_kernel`] instead
    ///
    /// use [`PageTable::new_bare`] to create a completly empty page table,
    /// or use [`PageTable::new_allocated`] to create one with root allocated
    pub fn new_bare(page_table: PageTable) -> Self {
        Self {
            page_table,
            areas: Vec::new(),
            user_stack_area: None,
            user_heap_area: None,
            user_stack_base: 0,
            user_heap_base: 0,
        }
    }

    /// get token, which will be written into satp
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// switch into this memory set
    #[inline(always)]
    pub unsafe fn activate(&mut self) {
        unsafe {
            self.page_table.activate();
        }
    }

    /// translate va into pa
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.page_table.translate_va(va)
    }

    /// push a map area into current memory set
    /// load data if provided
    pub fn push_area(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        info!(
            "push_area: [{:#X}, {:#X})",
            map_area.vpn_range().start().0 << PAGE_WIDTH,
            map_area.vpn_range().end().0 << PAGE_WIDTH
        );
        map_area.map_each(&mut self.page_table);
        if let Some(data) = data {
            map_area.load_data(&mut self.page_table, data);
        }
        self.areas.push(map_area); // bind life cycle
    }

    #[cfg(feature = "qemu")]
    fn map_mmio(mapping: &mut Mapping) {
        // 映射 PLIC
        let plic_va_start = VirtualAddress(PLIC_BASE);
        let plic_va_end = VirtualAddress(PLIC_BASE + 0x400000);
        mapping.map_defined(
            &(plic_va_start..plic_va_end),
            &(plic_va_start.physical_address_linear()..plic_va_end.physical_address_linear()),
            Flags::READABLE | Flags::WRITABLE,
        );

        // 映射 virtio disk mmio
        let virtio_va = VirtualAddress(VIRTIO0);
        let virtio_pa = VirtualAddress(VIRTIO0).physical_address_linear();
        mapping.map_one(
            VirtualPageNumber::floor(virtio_va),
            Some(PhysicalPageNumber::floor(virtio_pa)),
            Flags::WRITABLE | Flags::READABLE,
        );
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
        trace!("[memory_set] sp: {:#x}", crate::arch::regs::get_sp());
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
        KERNEL_SPACE_TOKEN.store(memory_set.token(), Ordering::Relaxed);
        memory_set
    }

    /// create a new memory set with kernel space mapped,
    pub fn new_with_kernel() -> Self {
        let mut memory_set = Self::new_bare(PageTable::new_bare());
        memory_set.page_table = PageTable::clone_from_other(&KERNEL_SPACE.lock().page_table);
        memory_set
    }

    /// map user_stack_area
    /// TODO: is lazy allocation necessary? currently we don't use lazy alloc
    pub fn map_user_stack(&mut self, start: usize, end: usize) {
        // FIXME: is using start correct?
        self.user_stack_base = start;
        let mut map_area = MapArea::new(
            start.into(),
            end.into(),
            MapType::Framed,
            map_permission!(U, R, W),
            MapAreaType::UserStack,
        );
        map_area.map_each(&mut self.page_table);
        self.user_stack_area = Some(map_area);
    }

    /// map user_heap_area lazily
    pub fn map_user_heap(&mut self, start: usize, end: usize) {
        self.user_heap_base = start;
        let map_area = MapArea::new(
            start.into(),
            end.into(),
            MapType::Framed,
            map_permission!(U, R, W),
            MapAreaType::UserHeap,
        );
        // map_area.map_each(&mut self.page_table);
        self.user_heap_area = Some(map_area);
    }

    /// load data from elf file
    /// TODO: map trampoline?
    pub async fn load_from_elf(elf_file: Arc<dyn File>, elf_len: usize) -> ElfMemoryInfo {
        info!("[memory_set] load elf begins");
        let mut memory_set = Self::new_with_kernel();

        // // read elf header
        // const ELF_HEADER_SIZE: usize = 64;
        // let mut elf_buf = [0u8; ELF_HEADER_SIZE];
        // elf_file
        //     .read(0, ELF_HEADER_SIZE, &mut elf_buf)
        //     .await
        //     .unwrap();
        // let elf_ph = xmas_elf::ElfFile::new(elf_buf.as_slice()).unwrap().header;
        // debug!("elf_header: {:?}, length = {}", elf_ph, elf_len);

        // // read all program header
        // let ph_entry_size = elf_header.pt2.ph_entry_size() as usize;
        // let ph_offset: usize = elf_header.pt2.ph_offset() as usize;
        // let ph_count = elf_header.pt2.ph_count() as usize;
        // let mut elf_buf = vec![0u8; ph_offset + ph_count * ph_entry_size];
        // elf_file
        //     .read(0, ph_offset + ph_count * ph_entry_size, &mut elf_buf)
        //     .await
        //     .unwrap();
        // let elf_ph = xmas_elf::ElfFile::new(elf_buf.as_slice()).unwrap();
        // debug!("elf_ph: {:?}", elf_ph);

        // read all data
        let mut elf_buf = vec![0u8; elf_len];
        elf_file.read(0, elf_buf.len(), &mut elf_buf).await.unwrap();
        let elf = xmas_elf::ElfFile::new(elf_buf.as_slice()).unwrap();

        // check: magic
        let magic = elf.header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf.header.pt2.ph_count();
        let mut end_vpn = VirtPageNum(0);

        // map pages by loaded program header
        info!("[memory_set] data loaded! start to map data pages");
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let map_area = MapArea::new(
                    (ph.virtual_addr() as usize).into(),
                    ((ph.virtual_addr() + ph.mem_size()) as usize).into(),
                    MapType::Framed,
                    map_permission!(U).merge_from_elf_flags(ph.flags()),
                    MapAreaType::ElfBinary,
                );
                end_vpn = map_area.vpn_range.end();
                memory_set.push_area(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }

        info!("[memory_set] elf data load complete! start to map user stack");
        let end_va: VirtAddr = end_vpn.into();
        let elf_entry = elf.header.pt2.entry_point() as usize;
        info!("[memory_set] entry: {:#x}", elf_entry);

        let user_stack_base: usize = usize::from(end_va) + PAGE_SIZE; // stack bottom
        let user_stack_end = user_stack_base + USER_STACK_SIZE; // stack top
        memory_set.map_user_stack(user_stack_base.into(), user_stack_end.into());

        info!("[memory_set] user stack mapped! start to map user heap");
        let user_heap_base: usize = user_stack_end + PAGE_SIZE;
        let user_heap_end: usize = user_heap_base + USER_HEAP_SIZE;
        memory_set.map_user_heap(user_heap_base, user_heap_end);

        info!("[memory_set] user heap mapped! elf load complete!");
        ElfMemoryInfo {
            memory_set,
            elf_entry,
            user_sp: user_stack_end, // stack grows downward
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
