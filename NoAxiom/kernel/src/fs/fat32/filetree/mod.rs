pub mod directory;
pub mod entry;
pub mod file;
pub mod fs_node;

use alloc::{boxed::Box, vec::Vec};
use core::{
    borrow::{Borrow, BorrowMut},
    future::Future,
    pin::Pin,
};

use fs_node::FSNode;

use super::error::FileError;

/// the node of the file tree with identifier `T`
pub struct FileNode<T, V> {
    inner: Box<dyn FSNode<T, V>>,
    children: Vec<Box<FileNode<T, V>>>,
}

impl<T, V> FileNode<T, V>
where
    T: PartialEq,
{
    /// create a new file node
    fn from(fs_node: Box<dyn FSNode<T, V>>) -> Self {
        Self {
            inner: fs_node,
            children: Vec::new(),
        }
    }

    /// get the identifier of the node
    pub fn ident(&self) -> T {
        self.inner.ident()
    }

    /// get the content of the node
    pub async fn content<'a>(&'a self) -> Pin<Box<dyn Future<Output = V> + Send + 'a>> {
        self.inner.content()
    }

    /// get the part of content of the node
    pub async fn part_content<'a>(&'a self, offset: usize, len: usize, buf: &'a mut [u8]) {
        self.inner.part_content(offset, len, buf).await
    }

    /// identity the node with the given identifier
    fn identify(&self, id: &T) -> bool {
        self.inner.identify(id)
    }

    /// add a child to the file node
    fn add_child(&mut self, child: Box<FileNode<T, V>>) {
        self.children.push(child);
    }

    /// get childrens' reference
    fn children_ref<'a>(&'a self) -> Vec<&'a FileNode<T, V>> {
        self.children.iter().map(|b| b.borrow()).collect()
    }

    /// get childrens' mutable reference
    fn children_mut<'a>(&'a mut self) -> Vec<&'a mut FileNode<T, V>> {
        self.children.iter_mut().map(|b| b.borrow_mut()).collect()
    }

    /// find the mutable reference of the node in the subtree of the given node
    /// with identifier, return `None` if not found
    fn find_mut<'a>(&'a mut self, id: &T) -> Option<&'a mut FileNode<T, V>> {
        let mut queue = vec![self];
        while let Some(node) = queue.pop() {
            if node.identify(id) {
                return Some(node);
            } else {
                for child in node.children_mut() {
                    queue.push(child);
                }
            }
        }
        None
    }

    /// find the reference of the node in the subtree of the given node with
    /// identifier, return `None` if not found
    pub fn find<'a>(&'a self, id: &T) -> Option<&'a FileNode<T, V>> {
        let mut queue = vec![self];
        while let Some(node) = queue.pop() {
            if node.identify(id) {
                return Some(node);
            } else {
                for child in node.children_ref() {
                    queue.push(child);
                }
            }
        }
        None
    }
}

pub struct FileTree<T, V>
where
    T: PartialEq + Eq + Clone,
    V: Clone,
{
    root: Box<FileNode<T, V>>,
}

impl<T, V> FileTree<T, V>
where
    T: PartialEq + Eq + Clone,
    V: Clone,
{
    pub async fn from(root: Box<dyn FSNode<T, V>>) -> Self {
        Self {
            root: Box::new(FileNode::from(root)),
        }
    }
    /// insert a node to the file tree, return `true` if success, `false` if
    /// not found the parent node
    pub fn insert(
        &mut self,
        parent_ident: &T,
        node: Box<dyn FSNode<T, V>>,
    ) -> Result<(), FileError> {
        let parent_node = self.root.find_mut(parent_ident);
        if let Some(parent_node) = parent_node {
            parent_node.add_child(Box::new(FileNode::from(node)));
            Ok(())
        } else {
            Err(FileError::FileNotFound)
        }
    }
    /// list the children of the node with the given identifier
    pub fn list(&self, ident: &T) -> Result<Vec<&FileNode<T, V>>, FileError> {
        let node = self.root.find(ident);
        if let Some(node) = node {
            Ok(node.children_ref())
        } else {
            Err(FileError::FileNotFound)
        }
    }
    /// find the node with the given identifier
    pub fn find(&self, ident: &T) -> Result<&FileNode<T, V>, FileError> {
        let node = self.root.find(ident);
        if let Some(node) = node {
            Ok(node)
        } else {
            Err(FileError::FileNotFound)
        }
    }
}
