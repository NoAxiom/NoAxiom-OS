#[macro_export]
macro_rules! dentry_default {
    ($dentry_struct:ident, $file_struct:ident) => {
        mod dentry {
            use alloc::{boxed::Box, sync::Arc};

            use super::$file_struct;
            use crate::{
                fs::vfs::basic::{
                    dentry::{Dentry, DentryMeta},
                    file::{File, FileMeta},
                    superblock::SuperBlock,
                },
                include::fs::InodeMode,
                syscall::SysResult,
            };

            pub struct $dentry_struct {
                meta: DentryMeta,
            }

            impl $dentry_struct {
                pub fn new(
                    parent: Option<Arc<dyn Dentry>>,
                    name: &str,
                    super_block: Arc<dyn SuperBlock>,
                ) -> Self {
                    Self {
                        meta: DentryMeta::new(parent, name, super_block),
                    }
                }
            }

            #[async_trait::async_trait]
            impl Dentry for $dentry_struct {
                fn meta(&self) -> &DentryMeta {
                    &self.meta
                }

                fn from_name(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
                    unreachable!(
                        "{} dentry should not have child",
                        stringify!($dentry_struct)
                    );
                }

                fn open(self: Arc<Self>) -> SysResult<Arc<dyn File>> {
                    Ok(Arc::new($file_struct::new(FileMeta::new(
                        self.clone(),
                        self.inode()?,
                    ))))
                }

                async fn create(
                    self: Arc<Self>,
                    _name: &str,
                    _mode: InodeMode,
                ) -> SysResult<Arc<dyn Dentry>> {
                    unreachable!("{} should not create child", stringify!($dentry_struct));
                }

                async fn symlink(self: Arc<Self>, _name: &str, _tar_name: &str) -> SysResult<()> {
                    unreachable!("{} should not create symlink", stringify!($dentry_struct));
                }
            }
        }
        pub use dentry::$dentry_struct;
    };
}

#[macro_export]
macro_rules! file_default {
    ($file_struct:ident, $read_block:item, $write_block:item) => {
        pub mod file {
            use alloc::{boxed::Box};
            use core::task::Waker;

            use include::errno::Errno;

            use crate::{
                fs::vfs::basic::file::{File, FileMeta},
                include::io::PollEvent,
                syscall::SyscallResult,
            };

            pub struct $file_struct {
                meta: crate::fs::vfs::basic::file::FileMeta,
            }

            impl $file_struct {
                pub fn new(meta: crate::fs::vfs::basic::file::FileMeta) -> Self {
                    Self { meta }
                }
            }

            #[async_trait::async_trait]
            #[allow(unused)]
            impl File for $file_struct {
                fn meta(&self) -> &FileMeta {
                    &self.meta
                }
                async fn base_readlink(&self, _buf: &mut [u8]) -> crate::syscall::SyscallResult {
                    unreachable!(concat!("readlink from ", stringify!($file_struct)));
                }
                async fn load_dir(&self) -> crate::syscall::SysResult<()> {
                    Err(Errno::ENOTDIR)
                }
                async fn delete_child(&self, _name: &str) -> crate::syscall::SysResult<()> {
                    Err(Errno::ENOSYS)
                }
                fn ioctl(&self, _cmd: usize, _arg: usize) -> crate::syscall::SyscallResult {
                    Err(Errno::ENOTTY)
                }
                fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
                    unimplemented!(concat!(stringify!($file_struct), "::poll is not supported"));
                }
                $read_block
                $write_block
            }
        }
        pub use file::$file_struct;
    };
}
