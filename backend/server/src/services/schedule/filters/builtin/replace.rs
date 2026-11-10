use crate::services::schedule::{ResponseFilter, stages::replace_word};

pub struct Replace {
    from: String,
    to: String,
    pending: String,
    prev: Option<char>,
}

impl Replace {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            pending: String::new(),
            prev: None,
        }
    }

    fn scan(&self, end: usize) -> String {
        let mut out = String::new();
        if self.from.is_empty() {
            out.push_str(&self.pending[..end]);
            return out;
        }
        let mut cursor = 0;
        while cursor < end {
            let Some(rel) = self.pending[cursor..end].find(&self.from) else {
                break;
            };
            let i = cursor + rel;
            let j = i + self.from.len();
            out.push_str(&self.pending[cursor..i]);

            let left = if i > 0 {
                self.pending[..i].chars().next_back() // M
            } else {
                self.prev
            };
            let right = self.pending[j..].chars().next(); // F
            if left.map_or(true, |c| !is_word(c)) && right.map_or(true, |c| !is_word(c)) {
                out.push_str(&self.to);
            } else {
                out.push_str(&self.pending[i..j]);
            }
            cursor = j;
        }
        out.push_str(&self.pending[cursor..end]);
        out
    }
}

impl ResponseFilter for Replace {
    fn push(&mut self, delta: &str) -> String {
        self.pending.push_str(delta);
        let keep = held_prefix_len(&self.pending, &self.from);
        let end = self.pending.len() - keep;
        let out = self.scan(end);
        if end > 0 {
            self.prev = self.pending[..end].chars().next_back(); // char before the held tail
        }
        self.pending.drain(..end);
        out
    }

    fn drain(&mut self) -> String {
        let out = self.scan(self.pending.len());
        self.pending.clear();
        self.prev = None;
        out
    }

    fn apply(&self, text: &str) -> String {
        replace_word(text, &self.from, &self.to)
    }
}

fn held_prefix_len(s: &str, pred: &str) -> usize {
    let max = pred.len().min(s.len());
    for n in (1..=max).rev() {
        if s.is_char_boundary(s.len() - n)
            && pred.as_bytes().starts_with(&s.as_bytes()[s.len() - n..])
        {
            return n;
        }
    }
    0
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
