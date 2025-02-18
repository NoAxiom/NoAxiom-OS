use bitflags::bitflags;

// sys_wait4
bitflags! {
    pub struct WaitOption: i32 {
        const WNOHANG = 1 << 0;
        const WUNTRACED = 1 << 1;
        const WCONTINUED = 1 << 3;
    }
}
#[derive(Debug, Clone, Copy)]
#[allow(unused)]
pub enum PidSel {
    Group(Option<usize>),
    Task(Option<usize>),
}
