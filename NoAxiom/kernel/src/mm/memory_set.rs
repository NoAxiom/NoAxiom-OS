use alloc::{sync::Arc, vec::Vec};

use lazy_static::lazy_static;

use super::{map_area::MapArea, page_table::PageTable};
use crate::{
    config::mm::{PAGE_SIZE, PAGE_WIDTH, USER_HEAP_SIZE, USER_STACK_SIZE},
    fs::{self, File},
    map_permission,
    mm::{
        address::{VirtAddr, VirtPageNum},
        map_area::MapAreaType,
        permission::MapType,
    },
    sync::mutex::SpinMutex,
};

lazy_static! {
    pub static ref KERNEL_SPACE: SpinMutex<MemorySet> =
        SpinMutex::new(MemorySet::init_kernel_space());
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
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
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
    pub unsafe fn activate(&mut self) {
        unsafe {
            self.page_table.activate();
        }
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

    /// create kernel space, used in [`KERNEL_SPACE`] initialization
    pub fn init_kernel_space() -> Self {
        extern "C" {
            fn stext();
            fn etext();
            fn srodata();
            fn erodata();
            fn sdata();
            fn edata();
            fn sbss();
            fn ebss();
        }
        let mut memory_set = MemorySet::new_bare();
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
        );
        memory_set
    }

    /// create a new memory set with kernel space mapped,
    pub fn new_with_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        let kernel_space = KERNEL_SPACE.lock();
        memory_set.page_table = PageTable::clone_from_other(&kernel_space.page_table);
        drop(kernel_space);
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
    /// TODO: use file to read elf
    /// TODO: map trampoline?
    pub async fn load_from_elf(elf_file: Arc<dyn File>) -> ElfMemoryInfo {
        info!("[memory_set] load elf begins");
        let mut memory_set = Self::new_with_kernel();
        let mut elf_data = [1u8; 0x100000]; // todo: use elf_header
        let _ = elf_file.read(0, elf_data.len(), &mut elf_data).await;
        let elf = xmas_elf::ElfFile::new(&elf_data).unwrap();
        info!("elf header: {:?}", elf.header);

        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut end_vpn = VirtPageNum(0);

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
