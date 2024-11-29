//! memory management

mod address;
mod frame;
mod heap;
mod kmm;
mod memory_set;
mod page_table;
mod permission;
mod pte;

pub fn init() {
    frame::init();
    heap::init();
    // todo: memory set init
    crate::println!("[kernel] memory management initialized.");
}

/// returns pte flags with multiple flag bits
#[macro_export]
macro_rules! pte_flags {
    ($($flag:ident),*) => {
        {
            let mut flags = PTEFlags::empty();
            $(flags |= PTEFlags::$flag;)*
            flags
        }
    };
}

/// returns map permission with multiple flag bits
#[macro_export]
macro_rules! map_permission {
    ($($flag:ident),*) => {
        {
            let mut flags = MapPermission::empty();
            $(flags |= MapPermission::$flag;)*
            flags
        }
    };
}
