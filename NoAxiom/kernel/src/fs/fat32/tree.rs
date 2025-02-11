//! N-Tree 实现

use alloc::{boxed::Box, vec::Vec};
use core::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
};

use async_trait::async_trait;

#[async_trait]
pub trait AsNode: Send + Sync {
    /// 标识类型，通过这个类型进行结点查找
    type Ident: Debug + Send + Sync;
    /// 结点附带的数据内容
    type Content: Send + Sync;
    /// 结点附带的数据内容的引用
    type ContentRef: Send + Sync;
    /// 对比标识，一致返回 `true`
    fn identify(&self, ident: &Self::Ident) -> bool;
    /// 返回结点的标识，`Clone` 语义
    fn ident(&self) -> Self::Ident;
    /// 返回结点附带的数据内容
    async fn content(&self) -> Self::Content;
    /// 返回结点附带的数据内容的引用
    async fn content_ref(&self) -> Self::ContentRef;
}

/// 结点
pub struct Node<T, C, R> {
    /// 内部数据
    inner: Box<dyn AsNode<Ident = T, Content = C, ContentRef = R> + Send + Sync>,
    /// 子结点
    children: Vec<Box<Node<T, C, R>>>,
}

impl<T, C, R> Node<T, C, R> {
    /// 创建一个空节点
    pub fn empty(
        inner: Box<dyn AsNode<Ident = T, Content = C, ContentRef = R> + Send + Sync>,
    ) -> Self {
        Self {
            inner,
            children: Vec::new(),
        }
    }
    /// 插入一个子结点
    pub fn insert(
        &mut self,
        inner: Box<dyn AsNode<Ident = T, Content = C, ContentRef = R> + Send + Sync>,
    ) {
        let node = Box::new(Node::empty(inner));
        self.children.push(node);
    }
    /// 删除一个子结点，如果成功返回这个结点的 [`Box`]
    pub fn remove(&mut self, index: usize) -> Option<Box<Node<T, C, R>>> {
        if index >= self.children.len() {
            None
        } else {
            Some(self.children.remove(index))
        }
    }
    /// 获得这个结点的内部数据的不可变引用
    pub fn inner(&self) -> &Box<dyn AsNode<Ident = T, Content = C, ContentRef = R> + Send + Sync> {
        &self.inner
    }
    /// 获取这个结点的内部数据的可变引用
    pub fn inner_mut(
        &mut self,
    ) -> &mut Box<dyn AsNode<Ident = T, Content = C, ContentRef = R> + Send + Sync> {
        &mut self.inner
    }
    /// 返回这个结点的某个子结点
    pub fn child<'a>(&'a self, index: usize) -> Option<&'a Node<T, C, R>> {
        if index >= self.children.len() {
            None
        } else {
            self.children.get(index).map(|b| b.borrow())
        }
    }
    /// 返回所有子结点的不可变引用
    pub fn children_ref<'a>(&'a self) -> Vec<&'a Node<T, C, R>> {
        self.children.iter().map(|b| b.borrow()).collect()
    }
    /// 返回所有子结点的可变引用
    pub fn children_iter_mut<'a>(&'a mut self) -> Vec<&'a mut Node<T, C, R>> {
        self.children.iter_mut().map(|b| b.borrow_mut()).collect()
    }
}

/// `N-Tree` 结构体
pub struct NTree<T, C, R> {
    root: Node<T, C, R>,
}

impl<T: Debug, C, R> NTree<T, C, R>
where
    T: Send + Sync,
    C: Send + Sync,
    R: Send + Sync,
{
    /// 根据根结点新建一棵 `N-Tree`
    pub fn new(
        root_inner: Box<dyn AsNode<Ident = T, Content = C, ContentRef = R> + Send + Sync>,
    ) -> Self {
        Self {
            root: Node::empty(root_inner),
        }
    }
    /// 查找结点，如果找到返回 `Some(&Node<T>)`
    pub fn find<S: Into<T>>(&self, ident: S) -> Option<&Node<T, C, R>> {
        let root = self.root.borrow();
        Self::traverse(root, &ident.into())
    }
    /// 遍历查找
    pub fn traverse<'a>(root: &'a Node<T, C, R>, ident: &T) -> Option<&'a Node<T, C, R>> {
        let mut queue = Vec::new();
        queue.push(root);
        while let Some(node) = queue.pop() {
            if node.inner.identify(ident) {
                return Some(node);
            } else {
                node.children_ref().iter().for_each(|n| queue.push(*n));
            }
        }
        None
    }
    /// 查找结点，如果找到返回 `Some(&mut Node<T>)`
    pub fn find_mut<S: Into<T>>(&mut self, ident: S) -> Option<&mut Node<T, C, R>> {
        let root = self.root.borrow_mut();
        Self::traverse_mut(root, &ident.into())
    }
    /// 层序遍历
    pub fn traverse_mut<'a>(
        root: &'a mut Node<T, C, R>,
        ident: &T,
    ) -> Option<&'a mut Node<T, C, R>> {
        let mut queue = Vec::new();
        queue.push(root);
        while let Some(node) = queue.pop() {
            if node.inner().identify(ident) {
                return Some(node);
            }
            for child in node.children_iter_mut() {
                queue.push(child);
            }
        }
        None
    }
    /// 返回根结点的不可变引用
    pub fn root(&self) -> &Node<T, C, R> {
        self.root.borrow()
    }
    /// 返回根节点的可变引用
    pub fn root_mut(&mut self) -> &mut Node<T, C, R> {
        self.root.borrow_mut()
    }
    /// 列出某个结点的所有子结点
    pub fn list<'a, S: Into<T>>(&'a self, ident: S) -> Vec<&'a Node<T, C, R>> {
        if let Some(node) = self.find(ident) {
            node.children_ref()
        } else {
            Vec::new()
        }
    }
    /// 列出某个结点的某个子结点
    pub fn list_one<'a, S: Into<T>>(&'a self, ident: S, index: usize) -> Option<&'a Node<T, C, R>> {
        if let Some(node) = self.find(ident) {
            node.child(index)
        } else {
            None
        }
    }
    // /// 删除某个结点，以及以这个结点为根结点的子树
    // pub fn remove(&mut self, ident: &T) -> Option<Box<Node<T, C, R>>> {
    //     todo!()
    // }
}
