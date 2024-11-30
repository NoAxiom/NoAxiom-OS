use alloc::vec::Vec;

use super::{map_area::MapArea, page_table::PageTable};
use crate::{
    config::mm::{PAGE_SIZE, USER_HEAP_SIZE, USER_STACK_SIZE},
    map_permission,
    mm::{
        address::{VirtAddr, VirtPageNum},
        map_area::MapAreaType,
        permission::MapType,
    },
};

/// elf load result information
pub struct ElfMemoryInfo {
    pub memory_set: MemorySet,
    pub elf_entry: usize,
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
    pub fn new() -> Self {
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
        map_area.map_each(&mut self.page_table);
        if let Some(data) = data {
            map_area.load_data(&mut self.page_table, data);
        }
        self.areas.push(map_area); // bind life cycle
    }

    /// map user_stack_area lazily
    pub fn map_user_stack(&mut self, start: usize, end: usize) {
        self.user_stack_base = start;
        self.user_stack_area = Some(MapArea::new(
            start.into(),
            end.into(),
            MapType::Framed,
            map_permission!(U, R, W),
            MapAreaType::UserStack,
        ));
    }

    /// map user_heap_area lazily
    pub fn map_user_heap(&mut self, start: usize, end: usize) {
        self.user_heap_base = start;
        self.user_heap_area = Some(MapArea::new(
            start.into(),
            end.into(),
            MapType::Framed,
            map_permission!(U, R, W),
            MapAreaType::UserHeap,
        ));
    }

    /// load data from elf file
    /// TODO: use file to read elf
    /// TODO: map trampoline
    pub fn new_from_elf(elf_data: &[u8]) -> ElfMemoryInfo {
        let mut memory_set = Self::new();
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();

        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut end_vpn = VirtPageNum(0);

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

        let end_va: VirtAddr = end_vpn.into();
        let elf_entry = elf.header.pt2.entry_point() as usize;

        let user_stack_base: usize = usize::from(end_va) + PAGE_SIZE; // stack bottom
        let user_stack_end = user_stack_base + USER_STACK_SIZE; // stack top
        memory_set.map_user_stack(user_stack_base.into(), user_stack_end.into());

        let user_heap_base: usize = user_stack_end + PAGE_SIZE;
        let user_heap_end: usize = user_heap_base + USER_HEAP_SIZE;
        memory_set.map_user_heap(user_heap_base, user_heap_end);

        ElfMemoryInfo {
            memory_set,
            elf_entry,
        }
    }
}
