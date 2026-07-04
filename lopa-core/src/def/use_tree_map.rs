use la_arena::{Arena, Idx};

use crate::{
    def::{AstId, ErasedAstId, ast_id_map, ast_map, hir},
    parsing::{self, AstNode},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct UseTreeId(Idx<parsing::NodeId>);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UseTreeMap {
    arena: Arena<parsing::NodeId>,
}

impl UseTreeMap {
    fn insert<'a>(&mut self, use_tree: parsing::UseTree<'a>) -> UseTreeId {
        UseTreeId(self.arena.alloc(use_tree.id()))
    }
}

impl std::ops::Index<UseTreeId> for UseTreeMap {
    type Output = parsing::NodeId;

    fn index(&self, index: UseTreeId) -> &Self::Output {
        &self.arena[index.0]
    }
}

#[salsa::tracked]
fn use_tree_map<'db>(
    db: &'db dyn salsa::Database,
    module: hir::Module<'db>,
    //required instead of `AstId` because rust can't handle 'static lifetime inside `parsing::UseItem`
    use_id: ErasedAstId,
) -> UseTreeMap {
    let map = UseTreeMap::default();
    let file = module.file(db);
    let node_id = ast_map(db, file)[AstId::<parsing::UseItem<'static>>::from(use_id)];
    let parse = file.parse(db);
    let node = parse.tree(db).get(node_id).unwrap();
    map
}
