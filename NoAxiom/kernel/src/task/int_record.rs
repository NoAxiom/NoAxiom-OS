use alloc::string::String;

#[derive(Debug, Clone)]
pub struct InterruptRecorder {
    pub cnt: [usize; 64],
}

impl InterruptRecorder {
    pub const fn new() -> Self {
        Self { cnt: [0; 64] }
    }

    pub fn inc(&mut self, idx: usize) {
        if idx < self.cnt.len() {
            self.cnt[idx] += 1;
        }
    }

    pub fn reset(&mut self) {
        self.cnt = [0; 64];
    }

    pub fn get(&self, idx: usize) -> usize {
        if idx < self.cnt.len() {
            self.cnt[idx]
        } else {
            0
        }
    }

    pub fn get_proc_interrupts(&self) -> String {
        let mut s = String::new();
        for (i, &cnt) in self.cnt.iter().enumerate() {
            if cnt > 0 {
                s.push_str(&format!("{}: {}\n", i, cnt));
            }
        }
        s
    }
}

impl Default for InterruptRecorder {
    fn default() -> Self {
        Self::new()
    }
}
