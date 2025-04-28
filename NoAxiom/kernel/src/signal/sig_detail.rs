#[derive(Copy, Clone, Debug)]
#[allow(unused)]
pub enum SigDetail {
    None,
    Child(SigChildDetail),
    Kill(SigKillDetail),
}

#[derive(Copy, Clone, Debug)]
#[allow(unused)]
pub struct SigChildDetail {
    // pub _si_uid: u32,     // Real user ID of sending process
    pub pid: u32,            // Sending process ID
    pub status: Option<i32>, // Exit value or signal
    pub utime: Option<i32>,  // User time consumed
    pub stime: Option<i32>,  // System time consumed
}

#[derive(Copy, Clone, Debug)]
#[allow(unused)]
pub struct SigKillDetail {
    pub pid: usize,
}
