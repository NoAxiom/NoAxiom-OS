use arch::TrapContext;

pub struct TaskTrapContext {
    pub cx: TrapContext,
    pub int_en: bool,
}

impl TaskTrapContext {
    pub fn new(cx: TrapContext, int_en: bool) -> Self {
        Self { cx, int_en }
    }
    pub fn cx(&self) -> &TrapContext {
        &self.cx
    }
    pub fn cx_mut(&mut self) -> &mut TrapContext {
        &mut self.cx
    }
}
