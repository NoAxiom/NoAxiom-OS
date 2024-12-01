//! memory management

mod address;
mod frame;
mod heap;
mod map_area;
mod memory_set;
mod page_table;
mod permission;
mod pte;

pub use memory_set::MemorySet;

pub fn init() {
    frame::init();
    heap::init();
}

/// returns pte flags with multiple flag bits
#[macro_export]
macro_rules! pte_flags {
    ($($flag:ident),*) => {
        {
            let mut flags = crate::mm::pte::PTEFlags::empty();
            $(flags |= crate::mm::pte::PTEFlags::$flag;)*
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
