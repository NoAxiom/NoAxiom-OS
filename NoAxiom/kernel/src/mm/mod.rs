mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub fn init() {
    heap_allocator::init_heap();
}
