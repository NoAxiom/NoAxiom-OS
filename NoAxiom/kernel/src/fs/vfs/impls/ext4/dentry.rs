use alloc::{boxed::Box, sync::Arc};

use arch::{Arch, ArchInt};
use async_trait::async_trait;
use ksync::assert_no_lock;

use super::{
    file::{Ext4Dir, Ext4File},
    inode::{Ext4DirInode, Ext4FileInode},
};
use crate::{
    fs::vfs::{
        basic::{
            dentry::{Dentry, DentryMeta},
            file::File,
            superblock::SuperBlock,
        },
        impls::ext4::{fs_err, superblock::Ext4SuperBlock},
    },
    include::{fs::InodeMode, result::Errno},
    syscall::SysResult,
};

pub struct Ext4Dentry {
    meta: DentryMeta,
}

impl Ext4Dentry {
    pub fn new(
        parent: Option<Arc<dyn Dentry>>,
        name: &str,
        super_block: Arc<dyn SuperBlock>,
    ) -> Self {
        Self {
            meta: DentryMeta::new(parent, name, super_block),
        }
    }

    pub fn into_dyn(self: Arc<Self>) -> Arc<dyn Dentry> {
        self.clone()
    }
}

#[async_trait]
impl Dentry for Ext4Dentry {
    fn meta(&self) -> &DentryMeta {
        &self.meta
    }

    fn from_name(self: Arc<Self>, name: &str) -> Arc<dyn Dentry> {
        let super_block = self.meta.super_block.clone();
        Arc::new(Self::new(Some(self), name, super_block))
    }

    fn open(self: Arc<Self>) -> SysResult<Arc<dyn File>> {
        let inode = self.inode()?;
        match inode.file_type() {
            InodeMode::DIR => Ok(Arc::new(Ext4Dir::new(
                self.clone(),
                inode
                    .downcast_arc::<Ext4DirInode>()
                    .map_err(|_| Errno::EIO)?,
            ))),
            InodeMode::FILE => Ok(Arc::new(Ext4File::new(
                self.clone(),
                inode
                    .downcast_arc::<Ext4FileInode>()
                    .map_err(|_| Errno::EIO)?,
            ))),
            InodeMode::LINK => todo!("link file!"),
            _ => Err(Errno::EINVAL),
        }
    }

    /*
    if Path::from_cd_or_create just create a negative dentry
    Dentry::create calls self.inode()? will panic,
    so when calling Dentry::create, we are sure that the dentry is not negative,
    and then we open ALL the dentry from root to here
     */
    async fn create(self: Arc<Self>, name: &str, mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        assert_no_lock!();
        assert!(Arch::is_interrupt_enabled());
        let inode = self.inode()?;
        assert!(inode.file_type() == InodeMode::DIR);
        let downcast_inode = inode
            .clone()
            .downcast_arc::<Ext4DirInode>()
            .map_err(|_| Errno::EIO)?;
        let this_inode_num = downcast_inode.get_inode().lock().inode_num;
        let super_block = self.clone().into_dyn().super_block();
        trace!("[ext4] try to get lock super block");
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs()
            .await;
        trace!("[ext4] get lock super block succeed!");
        let self_path = self.clone().into_dyn().path()?.as_string();
        let child_path = if self_path != "/" {
            format!("{}/{}", self_path, name)
        } else {
            format!("/{}", name)
        };
        // if ext4
        //     .dir_find_entry(
        //         this_inode_num,
        //         &name,
        //         &mut Ext4DirSearchResult::new(Ext4DirEntry::default()),
        //     )
        //     .await
        //     .is_ok()
        // {
        //     warn!("file \"{}\" exists, ignore create!", child_path);
        //     let res = self.into_dyn().get_child(name).unwrap();
        //     trace!("res file type: {:?}", res.inode().unwrap().file_type());
        //     return Ok(res);
        // }
        assert_no_lock!();
        assert!(Arch::is_interrupt_enabled());
        if mode.contains(InodeMode::FILE) {
            debug!(
                "[ext4] create file: {}, parent_inode: {}",
                child_path, this_inode_num
            );
            let new_file_inode = ext4
                .create(this_inode_num, name, 0x8000)
                .await
                .map_err(fs_err)?;
            trace!("[ext4] drop ext4");
            drop(ext4);
            // let inode_type = new_file_inode.inode.file_type();
            // debug!("new file inode type: {:?}", inode_type);
            let new_inode = Ext4FileInode::new(super_block.clone(), new_file_inode);
            Ok(self.into_dyn().add_child(name, Arc::new(new_inode)))
        } else if mode.contains(InodeMode::DIR) {
            debug!("[ext4] create dir: {}", child_path);
            ext4.dir_mk(&child_path).await.map_err(fs_err)?;
            let inode_num = ext4.ext4_dir_open(&child_path).await.map_err(fs_err)?;
            let new_dir_inode = ext4.get_inode_ref(inode_num).await;
            trace!("[ext4] drop ext4");
            drop(ext4);
            let new_inode = Ext4DirInode::new(super_block.clone(), new_dir_inode);
            Ok(self.into_dyn().add_child(name, Arc::new(new_inode)))
        } else {
            Err(Errno::EINVAL)
        }
    }
    async fn symlink(self: Arc<Self>, name: &str, tar_name: &str) -> SysResult<()> {
        debug!("[ext4] create link: {}", name);
        let inode = self.inode()?;
        assert!(inode.file_type() == InodeMode::FILE);

        let inode = self.clone().into_dyn().parent().unwrap().inode()?;
        let downcast_inode = inode
            .clone()
            .downcast_arc::<Ext4DirInode>()
            .map_err(|_| Errno::EIO)?;
        let parent_inode_num = downcast_inode.get_inode().lock().inode_num;
        let super_block = self.clone().into_dyn().super_block();
        assert_no_lock!();
        assert!(Arch::is_interrupt_enabled());
        let mut ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs()
            .await;
        debug!("[ext4] get lock super block succeed!");

        // todo: because of the ext4_rs `fuse_symlimk` MUST CREATE a file, so we delete
        // the existed file first
        assert_no_lock!();
        assert!(Arch::is_interrupt_enabled());
        let mut inode = ext4.get_inode_ref(parent_inode_num).await;

        debug!(
            "[ext4] remove entry: {}, parent_inode: {}",
            name, parent_inode_num
        );

        assert_no_lock!();
        assert!(Arch::is_interrupt_enabled());
        ext4.dir_remove_entry(&mut inode, &name)
            .await
            .map_err(fs_err)?;

        debug!("[ext4] remove entry succeed!");

        assert_no_lock!();
        assert!(Arch::is_interrupt_enabled());
        ext4.fuse_symlink(parent_inode_num as u64, name, tar_name)
            .await
            .map_err(fs_err)?;
        debug!("[ext4] symlink succeed!");
        Ok(())
    }
}
