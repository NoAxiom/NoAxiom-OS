/// hart related arch trait
pub trait ArchAsm {
    fn set_hartid(hartid: usize);
    fn get_hartid() -> usize;
    fn set_idle();
}