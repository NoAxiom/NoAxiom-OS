# CFS调度器设计

## 单核的cfs调度

通过vruntime进行运行时间的维护

子进程继承父进程的vruntime，防止大量fork导致的饥饿

长时间等待的进程在唤醒后，需要进行cpu时间片的补偿。我们的做法是使用urgent队列放置所有刚加入/刚唤醒的任务，它们的优先级是最高的，urgent任务之间按照先来先服务进行调度。

防止usize溢出导致的vruntime比较错误：使用差值进行比较，只要不超过`usize::MAX >> 1`就是正常比较的。

```rust
#[inline(always)]
pub fn less(a: usize, b: usize) -> bool {
    (isize)(a - b) < 0isize
}
```

## 多核的cfs调度

todo
