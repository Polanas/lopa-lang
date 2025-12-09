#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BytePos(pub usize);

impl BytePos {
    pub fn shift(&mut self, ch: char) {
        self.0 += ch.len_utf8();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Span {
    pub start: BytePos,
    pub end: BytePos,
}

impl Span {
    pub fn new(start: BytePos, end: BytePos) -> Self {
        Self { start, end }
    }

    pub fn empty() -> Self {
        Self {
            start: BytePos(0),
            end: BytePos(0),
        }
    }
}

pub struct WithSpan<T> {
    pub value: T,
    pub span: Span,
}

impl<T> WithSpan<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn empty(value: T) -> Self {
        Self {
            value,
            span: Span::empty(),
        }
    }
}
