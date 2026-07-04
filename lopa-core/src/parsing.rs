#[macro_use]
mod lexer;
mod ast;
mod parser;
mod token_set;

use std::hash::Hash;

pub use ast::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Tree(parser::Tree);

impl std::ops::Deref for Tree {
    type Target = parser::Tree;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for Tree {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.walk().with_depths().for_each(|(_, n)| {
            n.value().hash(state);
            n.range().hash(state);
        })
    }
}

unsafe impl Sync for Tree {}
unsafe impl Send for Tree {}

pub type NodeId = parser::NodeId;
pub type Node<'a> = parser::Node<'a>;
pub type Children<'a> = parser::Children<'a>;

pub use lexer::Syntax;
pub use parser::ParseError;

pub fn parse(input: &str) -> (Tree, Vec<ParseError>) {
    let (tree, errors) = parser::parse(input);
    (Tree(tree), errors)
}

#[salsa::tracked]
pub struct Parse<'db> {
    #[returns(ref)]
    pub tree: Tree,
    #[returns(ref)]
    pub errors: Vec<ParseError>,
}

impl<'db> Parse<'db> {
    pub fn file<'a>(&'a self, db: &'db dyn salsa::Database) -> Option<File<'a>> {
        File::cast(self.tree(db).first()?)
    }

    pub fn cast<'a, A: AstNode<'a>>(
        &'a self,
        db: &'db dyn salsa::Database,
        id: NodeId,
    ) -> Option<A> {
        self.tree(db).get(id).and_then(A::cast)
    }
}
