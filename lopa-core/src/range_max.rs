use rowan::TextRange;

pub trait RangeMax {
    fn max(self, other: Self) -> Self;
}

impl RangeMax for TextRange {
    fn max(self, other: Self) -> Self {
        if self.start() > other.start() {
            self
        } else {
            other
        }
    }
}
