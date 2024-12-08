//! memory management

pub mod address;
mod frame;
mod heap;
pub mod map_area;
pub mod memory_set;
pub mod page_table;
pub mod permission;
pub mod pte;

pub fn mm_init() {
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
