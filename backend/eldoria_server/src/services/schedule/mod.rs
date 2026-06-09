pub mod filters;
pub mod stages;
mod types;

pub use types::*;

use crate::services::chat::PromptMessage;

pub struct Schedule {
    stages: Vec<Box<dyn Stage>>,
}

impl Schedule {
    pub fn new(stages: Vec<Box<dyn Stage>>) -> Self {
        Self { stages }
    }

    pub fn outbound(&self, prompt: &mut Vec<PromptMessage>, cx: &StageCx) -> FilterChain {
        let mut filters: Vec<_> = self.stages.iter().map(|s| s.outbound(prompt, cx)).collect();
        filters.reverse(); // onion: inbound unwinds outbound order
        FilterChain { filters }
    }
}

pub struct FilterChain {
    filters: Vec<Box<dyn ResponseFilter>>,
}

impl FilterChain {
    pub fn push(&mut self, delta: &str) -> String {
        let mut buf = delta.to_owned();
        for f in &mut self.filters {
            buf = f.push(&buf);
            if buf.is_empty() {
                break;
            }
        }
        buf
    }

    pub fn finish(&mut self) -> String {
        // Flush each filter in order; its output still has to pass through
        // everything downstream of it (which is flushed on its own turn).
        let mut out = String::new();
        for i in 0..self.filters.len() {
            let mut chunk = self.filters[i].finish();
            for j in i + 1..self.filters.len() {
                chunk = self.filters[j].push(&chunk);
            }
            out.push_str(&chunk);
        }
        out
    }
}
