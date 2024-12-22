use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct CloneFlags: usize{
         /* set if VM shared between processes */
         const VM = 0x0000100;
         /* set if fs info shared between processes */
         const FS = 0x0000200;
         /* set if open files shared between processes */
         const FILES = 0x0000400;
         /* set if signal handlers and blocked signals shared */
         const SIGHAND = 0x00000800;
         /* set if we want to have the same parent as the cloner */
         const PARENT = 0x00008000;
         /* Same thread group? */
         const THREAD = 0x00010000;
         /* share system V SEM_UNDO semantics */
         const SYSVSEM = 0x00040000;
         /* create a new TLS for the child */
         const SETTLS = 0x00080000;
         /* set the TID in the parent */
         const PARENT_SETTID = 0x00100000;
         /* clear the TID in the child */
         const CHILD_CLEARTID = 0x00200000;
         /* Unused, ignored */
         const CLONE_DETACHED = 0x00400000;
         /* set the TID in the child */
         const CHILD_SETTID = 0x01000000;
         /* clear child signal handler */
         const CHILD_CLEAR_SIGHAND = 0x100000000;
    }
}
