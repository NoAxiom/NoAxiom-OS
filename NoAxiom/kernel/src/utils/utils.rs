#[inline(always)]
#[allow(unused)]
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[inline(always)]
pub fn is_aligned(addr: usize, align: usize) -> bool {
    (addr & (align - 1)) == 0
}

#[inline(always)]
pub fn align_offset(offset: usize, align: usize) -> (usize, usize) {
    let offset_aligned = offset & !(align - 1);
    let offset_inner = offset - offset_aligned;
    (offset_aligned, offset_inner)
}

/// Generate a **Random** number at an **EXTREMELY** efficient way
pub fn random() -> u64 {
    static mut SEED: u64 = 253496567482;
    unsafe {
        SEED = SEED * 1103515245 + 12345;
        SEED
    }
}

pub fn global_alloc() -> u64 {
    use core::sync::atomic::AtomicU64;
    static mut ALLOC: AtomicU64 = AtomicU64::new(0);
    unsafe {
        ALLOC.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        ALLOC.load(core::sync::atomic::Ordering::SeqCst)
    }
}

#[inline(always)]
pub fn random_fill(buf: &mut [u8]) -> usize {
    for i in 0..buf.len() {
        buf[i] = (random() & 0xFF) as u8;
    }
    buf.len()
}
