use std::sync::Mutex;

#[derive(Default)]
pub struct LogBus {
    lines: Mutex<Vec<String>>,
}

impl LogBus {
    pub fn clear(&self) {
        if let Ok(mut g) = self.lines.lock() {
            g.clear();
        }
    }

    pub fn push(&self, line: String) {
        if let Ok(mut g) = self.lines.lock() {
            g.push(line);
        }
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines.lock().map(|g| g.clone()).unwrap_or_default()
    }
}
