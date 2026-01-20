use crate::ast;

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

    pub const fn empty() -> Self {
        Self {
            start: BytePos(0),
            end: BytePos(0),
        }
    }

    pub fn union(&self, other: Self) -> Self {
        Self {
            start: other.start.min(self.start),
            end: other.end.max(self.end),
        }
    }
}

pub trait Spanned {
    fn span(&self) -> Span;
}

#[derive(Debug, PartialEq, Clone)]
pub struct WithSpan<T> {
    pub value: T,
    pub span: Span,
}

impl<T> WithSpan<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub const fn empty(value: T) -> Self {
        Self {
            value,
            span: Span::empty(),
        }
    }

    pub fn map<R>(self, map: impl FnOnce(T) -> R) -> WithSpan<R> {
        WithSpan::new(map(self.value), self.span)
    }

    pub fn as_ref(&self) -> WithSpan<&T> {
        WithSpan::new(&self.value, self.span)
    }

    pub fn union(&self, other: &Self) -> Span {
        self.span.union(other.span)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

pub struct LineOffsets {
    offsets: Vec<usize>,
    len: usize,
}

impl LineOffsets {
    pub fn new(data: &str) -> Self {
        let mut offsets = vec![0];

        for (i, val) in data.bytes().enumerate() {
            if val == b'\n' {
                offsets.push((i + 1) as _);
            }
        }

        Self {
            offsets,
            len: data.len(),
        }
    }

    pub fn line(&self, offset: BytePos) -> usize {
        let offset = offset.0;
        assert!(offset <= self.len);
        match self.offsets.binary_search(&offset) {
            Ok(line) => line,
            Err(line) => line,
        }
    }
}
