use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;

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
        let inode = self.inode()?;
        let downcast_inode = inode
            .clone()
            .downcast_arc::<Ext4DirInode>()
            .map_err(|_| Errno::EIO)?;
        let this_inode_num = downcast_inode.get_inode().lock().inode_num;
        assert!(inode.file_type() == InodeMode::DIR);
        let super_block = self.clone().into_dyn().super_block();
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs();
        let self_path = self.clone().into_dyn().path().as_string();
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
        if mode.contains(InodeMode::FILE) {
            debug!(
                "create file: {}, parent_inode: {}",
                child_path, this_inode_num
            );
            let new_file_inode = ext4
                .create(this_inode_num, name, 0x8000)
                .await
                .map_err(fs_err)?;
            // let inode_type = new_file_inode.inode.file_type();
            // debug!("new file inode type: {:?}", inode_type);
            let new_inode = Ext4FileInode::new(super_block.clone(), new_file_inode);
            Ok(self.into_dyn().add_child(name, Arc::new(new_inode)))
        } else if mode.contains(InodeMode::DIR) {
            debug!("create dir: {}", child_path);
            ext4.dir_mk(&child_path).await.map_err(fs_err)?;
            let inode_num = ext4.ext4_dir_open(&child_path).await.map_err(fs_err)?;
            let new_dir_inode = ext4.get_inode_ref(inode_num).await;
            let new_inode = Ext4DirInode::new(super_block.clone(), new_dir_inode);
            Ok(self.into_dyn().add_child(name, Arc::new(new_inode)))
        } else {
            Err(Errno::EINVAL)
        }
    }
}
