use async_trait::async_trait;

/// TCP/UDP or other socket should implement this trait
#[async_trait]
trait Socket {}
