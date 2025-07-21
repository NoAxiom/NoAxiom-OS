use crate::archs::arch_handle_irq;

#[inline]
pub fn handle_irq() {
    arch_handle_irq();
}
