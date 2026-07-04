#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    pub fn new(lo: u32, hi: u32) -> Self {
        Self { lo, hi }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }
}

pub struct SourceMap {
    src: String,
    line_starts: Vec<u32>,
}

impl SourceMap {
    pub fn new(src: String) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self { src, line_starts }
    }

    pub fn line_col(&self, pos: u32) -> (usize, usize) {
        match self.line_starts.binary_search(&pos) {
            Ok(idx) => (idx + 1, 1),
            Err(idx) => {
                let line_start = self.line_starts[idx - 1];
                let col = (pos - line_start) as usize + 1;
                (idx, col)
            }
        }
    }

    pub fn snippet(&self, span: Span) -> &str {
        &self.src[span.lo as usize..span.hi as usize]
    }
}
