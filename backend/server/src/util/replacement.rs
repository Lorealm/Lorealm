use std::sync::Mutex;

struct Replace {
    from: String,
    to: String,
}

pub struct BulkReplace {
    source: Mutex<String>,
    operations: Mutex<Vec<Replace>>,
}

impl BulkReplace {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: Mutex::new(source.into()),
            operations: Mutex::new(Vec::new()),
        }
    }

    pub fn register(&self, from: impl Into<String>, to: impl Into<String>) -> &Self {
        if let Ok(mut ops) = self.operations.lock() {
            ops.push(Replace {
                from: from.into(),
                to: to.into(),
            });
        }

        self
    }

    pub fn execute(&self) -> String {
        if let Ok(mut source) = self.source.lock() {
            if let Ok(ops) = self.operations.lock() {
                for r in ops.iter() {
                    *source = source.replace(&r.from, &r.to);
                }
            }
            source.clone()
        } else {
            String::new()
        }
    }
}
