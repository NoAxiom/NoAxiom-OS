pub trait BaseFu {
    fn putchar(c: u8);
    fn getchar() -> u8;
    fn shutdown() -> !;
}
