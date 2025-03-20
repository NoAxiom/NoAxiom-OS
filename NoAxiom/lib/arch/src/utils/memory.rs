use config::mm::PAGE_WIDTH;

pub fn ppn_to_pa(ppn: usize) -> usize {
    ppn << PAGE_WIDTH
}

pub fn pa_to_ppn(pa: usize) -> usize {
    pa >> PAGE_WIDTH
}
