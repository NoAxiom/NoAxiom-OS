//! memory management

pub mod address;
pub mod bss;
pub mod frame;
pub mod heap;
pub mod map_area;
pub mod memory_set;
pub mod mmap_manager;
pub mod page_table;
pub mod permission;
pub mod user_ptr;
pub mod validate;

#[inline(always)]
pub fn kernel_space_init() {
    memory_set::KERNEL_SPACE.lock().memory_activate();
}

/// returns pte flags with multiple flag bits
#[macro_export]
macro_rules! pte_flags {
    ($($flag:ident),*) => {
        {
            let mut flags = arch::MappingFlags::empty();
            $(flags |= arch::MappingFlags::$flag;)*
            flags
        }
    };
}

/// returns map permission with multiple flag bits
#[macro_export]
macro_rules! map_permission {
    ($($flag:ident),*) => {
        {
            let mut flags = crate::mm::permission::MapPermission::empty();
            $(flags |= crate::mm::permission::MapPermission::$flag;)*
            flags
        }
    };
}
