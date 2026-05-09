use crate::parsing::lexer::Syntax;

/// A bit-set of `Syntax`
#[derive(Clone, Copy)]
pub(crate) struct TokenSet(u128);

const _: () = const {
    assert!(Syntax::KEYWORD_LAST as u16 <= 128);
};

impl TokenSet {
    pub(crate) const fn new(kinds: &[Syntax]) -> TokenSet {
        let mut res = 0u128;
        let mut i = 0;
        while i < kinds.len() {
            res |= mask(kinds[i]);
            i += 1;
        }
        TokenSet(res)
    }

    pub(crate) const fn union(self, other: TokenSet) -> TokenSet {
        TokenSet(self.0 | other.0)
    }

    pub(crate) const fn contains(&self, kind: Syntax) -> bool {
        self.0 & mask(kind) != 0
    }
}

const fn mask(kind: Syntax) -> u128 {
    1u128 << (kind as usize)
}

#[test]
fn token_set_works_for_tokens() {
    use crate::parsing::lexer::Syntax::*;
    let ts = TokenSet::new(&[EOF]);
    assert!(ts.contains(EOF));
}
