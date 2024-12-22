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
}
