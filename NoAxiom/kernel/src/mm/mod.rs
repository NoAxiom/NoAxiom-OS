//! memory management

pub mod map_area;
pub mod memory_set;
pub mod mmap_manager;
pub mod page_table;
pub mod permission;
pub mod shm;
pub mod user_ptr;
pub mod validate;

pub use memory::*;
use memory::{frame::global_frame_init, utils::kernel_va_to_pa};
use platform::memory::PHYS_MEMORY_END;

pub fn frame_init() {
    extern "C" {
        fn ekernel(); // virt address
    }
    let start_pa = kernel_va_to_pa(ekernel as usize);
    let end_pa = PHYS_MEMORY_END;
    global_frame_init(start_pa, end_pa);
}
