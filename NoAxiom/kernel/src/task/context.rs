use arch::TrapContext;
use ksync::cell::SyncUnsafeCell;

pub struct TaskContext {
    pub cx: SyncUnsafeCell<TrapContext>,
    pub int_en: bool,
}

impl TaskContext {
    pub fn new(cx: TrapContext, int_en: bool) -> Self {
        Self {
            cx: SyncUnsafeCell::new(cx),
            int_en,
        }
    }
    pub fn cx(&self) -> &TrapContext {
        self.cx.as_ref()
    }
    pub fn cx_mut(&self) -> &mut TrapContext {
        self.cx.as_ref_mut()
    }
}
