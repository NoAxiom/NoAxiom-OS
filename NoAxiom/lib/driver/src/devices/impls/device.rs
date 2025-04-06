use alloc::boxed::Box;

use include::errno::Errno;

pub type DevResult<T> = Result<T, Errno>;

#[async_trait::async_trait]
#[allow(unused_variables)]
pub trait Device: Send + Sync {
    fn device_name(&self) -> &'static str;
    fn handle_interrupt(&self) -> DevResult<()> {
        unimplemented!("{} not implement handle_interrupt!", self.device_name())
    }
    async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        unimplemented!("{} not implement read!", self.device_name())
    }
    async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        unimplemented!("{} not implement read!", self.device_name())
    }
    async fn sync_all(&self) -> DevResult<()> {
        unimplemented!("{} not implement sync_all!", self.device_name())
    }
}
