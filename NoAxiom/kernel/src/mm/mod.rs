mod heap_allocator;

pub fn init() {
    heap_allocator::init_heap();
}
