use crate::services::schedule::ResponseFilter;

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

    /// Replace boundary-delimited matches in pending[..end]. Left context for a
    /// match at the front comes from `self.prev`; right context is the following
    /// char (None == end of stream == boundary, only reachable from finish()).
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
                self.pending[..i].chars().next_back()
            } else {
                self.prev
            };
            let right = self.pending[j..].chars().next();
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

    fn finish(&mut self) -> String {
        let out = self.scan(self.pending.len());
        self.pending.clear();
        self.prev = None;
        out
    }
}

fn held_prefix_len(s: &str, needle: &str) -> usize {
    let max = needle.len().min(s.len());
    for n in (1..=max).rev() {
        if s.is_char_boundary(s.len() - n)
            && needle.as_bytes().starts_with(&s.as_bytes()[s.len() - n..])
        {
            return n;
        }
    }
    0
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
