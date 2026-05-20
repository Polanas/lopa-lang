use std::{
    borrow::{Borrow, BorrowMut},
    hash::Hash,
    ops::{Deref, DerefMut},
};

pub type UstrIndexMap<V> =
    indexmap::IndexMap<UstrHash, V, identity_hash::BuildIdentityHasher<UstrHash>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, salsa::Update)]
pub struct UstrHash(pub ustr::Ustr);

impl From<ustr::Ustr> for UstrHash {
    fn from(value: ustr::Ustr) -> Self {
        Self(value)
    }
}

impl Borrow<str> for UstrHash {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Borrow<ustr::Ustr> for UstrHash {
    fn borrow(&self) -> &ustr::Ustr {
        &self.0
    }
}

impl DerefMut for UstrHash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for UstrHash {
    type Target = ustr::Ustr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl identity_hash::IdentityHashable for UstrHash {}

impl Hash for UstrHash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.precomputed_hash());
    }
}
