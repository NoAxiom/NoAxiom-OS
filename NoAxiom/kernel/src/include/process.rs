use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct CloneFlags: usize{
        /// set if VM shared between processes
        const VM = 0x0000100;
        /// set if fs info shared between processes
        const FS = 0x0000200;
        /// set if open files shared between processes
        const FILES = 0x0000400;
        /// set if signal handlers and blocked signals shared
        const SIGHAND = 0x00000800;
        /// set if we want to have the same parent as the cloner
        const PARENT = 0x00008000;
        /// Same thread group?
        const THREAD = 0x00010000;
        /// share system V SEM_UNDO semantics
        const SYSVSEM = 0x00040000;
        /// create a new TLS for the child
        const SETTLS = 0x00080000;
        /// set the TID in the parent
        const PARENT_SETTID = 0x00100000;
        /// clear the TID in the child
        const CHILD_CLEARTID = 0x00200000;
        /// Unused, ignored
        const CLONE_DETACHED = 0x00400000;
        /// set the TID in the child
        const CHILD_SETTID = 0x01000000;
        /// clear child signal handler
        const CHILD_CLEAR_SIGHAND = 0x100000000;
    }
}

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

/*
    # Auxiliary Vectors
    ELF auxiliary vectors are a mechanism to transfer certain kernel level
    information to the user processes. An example of such an information is the
    pointer to the system call entry point in the memory (AT_SYSINFO); this
    information is dynamic in nature and is only known after kernel has finished
    up loading. The information is passed on to the user processes by binary
    loaders which are part of the kernel subsystem itself; either built-in the
    kernel or a kernel module. Binary loaders convert a binary file, a program,
    into a process on the system. Usually there is a different loader for each
    binary format; thankfully there are not many binary formats - most of the
    linux based systems now use ELF binaries. ELF binary loader is defined in
    the following file /usr/src/linux/fs/binfmt_elf.c. The ELF loader parses the
    ELF file, maps the various program segments in the memory, sets up the entry
    point and initializes the process stack. It puts ELF auxiliary vectors on
    the process stack along with other information like argc, argv, envp. After
    initialization, a process' stack looks something like this:

    position            content                     size (bytes) + comment
    ------------------------------------------------------------------------
    stack pointer ->  [ argc = number of args ]     4
                    [ argv[0] (pointer) ]         4   (program name)
                    [ argv[1] (pointer) ]         4
                    [ argv[..] (pointer) ]        4 * x
                    [ argv[n - 1] (pointer) ]     4
                    [ argv[n] (pointer) ]         4   (= NULL)

                    [ envp[0] (pointer) ]         4
                    [ envp[1] (pointer) ]         4
                    [ envp[..] (pointer) ]        4
                    [ envp[term] (pointer) ]      4   (= NULL)

                    [ auxv[0] (Elf32_auxv_t) ]    8
                    [ auxv[1] (Elf32_auxv_t) ]    8
                    [ auxv[..] (Elf32_auxv_t) ]   8
                    [ auxv[term] (Elf32_auxv_t) ] 8   (= AT_NULL vector)

                    [ padding ]                   0 - 16

                    [ argument ASCIIZ strings ]   >= 0
                    [ environment ASCIIZ str. ]   >= 0

    (0xbffffffc)      [ end marker ]                4   (= NULL)

    (0xc0000000)      < bottom of stack >           0   (virtual)
    ------------------------------------------------------------------------
*/

pub mod auxv {
    #![allow(unused)]

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct AuxEntry(pub usize, pub usize);

    // ELF Auxiliary Vectors
    // https://articles.manugarg.com/aboutelfauxiliaryvectors.html
    pub const AT_NULL: usize = 0; // end of vector
    pub const AT_IGNORE: usize = 1; // entry should be ignored
    pub const AT_EXECFD: usize = 2; // file descriptor of program
    pub const AT_NOTELF: usize = 10; // program is not ELF
    pub const AT_PLATFORM: usize = 15; // string identifying CPU for optimizations
    pub const AT_BASE_PLATFORM: usize = 24; // string identifying real platform, may differ from AT_PLATFORM.
    pub const AT_HWCAP2: usize = 26; // extension of AT_HWCAP
    pub const AT_EXECFN: usize = 31; // filename of program
    pub const AT_PHDR: usize = 3; // program headers for program
    pub const AT_PHENT: usize = 4; // size of program header entry
    pub const AT_PHNUM: usize = 5; // number of program headers
    pub const AT_PAGESZ: usize = 6; // system page size
    pub const AT_BASE: usize = 7; // base address of interpreter
    pub const AT_FLAGS: usize = 8; // flags
    pub const AT_ENTRY: usize = 9; // entry point of program
    pub const AT_UID: usize = 11; // real uid
    pub const AT_EUID: usize = 12; // effective uid
    pub const AT_GID: usize = 13; // real gid
    pub const AT_EGID: usize = 14; // effective gid
    pub const AT_HWCAP: usize = 16; // arch dependent hints at CPU capabilities
    pub const AT_CLKTCK: usize = 17; // frequency at which times() increments
    pub const AT_SECURE: usize = 23; // secure mode boolean
    pub const AT_RANDOM: usize = 25; // address of 16 random bytes
}

pub mod robust_list {
    #[derive(Clone, Copy, Debug)]
    pub struct RobustList {
        pub head: usize,
        pub len: usize,
    }

    impl RobustList {
        pub const HEAD_SIZE: usize = 24;
    }

    impl Default for RobustList {
        fn default() -> Self {
            Self {
                head: 0,
                len: Self::HEAD_SIZE,
            }
        }
    }
}
