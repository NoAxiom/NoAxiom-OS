pub enum SysArgType {
    Int,
    Ptr,
    MutPtr,
    Void,
}

pub struct SysArgs {
    pub args: [usize; 6],
    pub atype: [SysArgType; 6],
}
