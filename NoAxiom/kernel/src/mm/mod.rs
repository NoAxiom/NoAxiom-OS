//! memory management

pub mod map_area;
pub mod memory_set;
pub mod mmap_manager;
pub mod page_table;
pub mod permission;
pub mod user_ptr;
pub mod validate;
pub use memory::*;

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
