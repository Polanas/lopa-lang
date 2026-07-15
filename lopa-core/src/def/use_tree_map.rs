use la_arena::{Arena, Idx};

use crate::parsing::{self, AstNode};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::Update)]
pub struct UseTreeId(Idx<parsing::NodeId>);

#[derive(Debug, Clone, Default, PartialEq, salsa::Update)]
pub struct UseTreeMap {
    arena: Arena<parsing::NodeId>,
}

impl UseTreeMap {
    pub(super) fn insert<'a>(&mut self, use_tree: parsing::UseTree<'a>) -> UseTreeId {
        UseTreeId(self.arena.alloc(use_tree.id()))
    }
}

impl std::ops::Index<UseTreeId> for UseTreeMap {
    type Output = parsing::NodeId;

    fn index(&self, index: UseTreeId) -> &Self::Output {
        &self.arena[index.0]
    }
}
