//! provide standard vfs structs, referrence from Phoenix OS
pub mod dentry;
pub mod file;
pub mod inode;
pub mod superblock;

/// all the file system should implement this trait
pub mod filesystem;

/// basic vfs macros
#[macro_use]
pub mod macros;

// todo: check all the WEAK or ARC pointers
