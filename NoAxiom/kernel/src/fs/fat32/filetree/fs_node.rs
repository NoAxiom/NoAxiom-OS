use alloc::boxed::Box;

use async_trait::async_trait;

#[async_trait]
/// the node of the file system tree
pub trait FSNode<T, V>: Sync + Send
where
    T: PartialEq,
{
    fn ident(&self) -> T;
    fn identify(&self, id: &T) -> bool {
        self.ident() == *id
    }
    async fn content(&self) -> V;
    async fn part_content<'a>(&'a self, offset: usize, len: usize, buf: &'a mut [u8]) {
        panic!("not implemented");
    }
}
