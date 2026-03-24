use rowan::{SyntaxNode, ast::AstNode, cursor};

use crate::lsp::{lexer::Syntax, parser};

macro_rules! impl_ast_node {
    ($type:ty,$token:ident) => {
        impl AstNode for $type {
            type Language = parser::Lang;

            fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
            where
                Self: Sized,
            {
                kind == Syntax::$token
            }

            fn cast(syntax: SyntaxNode<Self::Language>) -> Option<Self>
            where
                Self: Sized,
            {
                Self::can_cast(syntax.kind()).then(|| Self { syntax })
            }

            fn syntax(&self) -> &SyntaxNode<Self::Language> {
                &self.syntax
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockExpr {
    syntax: rowan::SyntaxNode<parser::Lang>,
}
impl_ast_node!(BlockExpr, BlockExpr);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFn {
    syntax: rowan::SyntaxNode<parser::Lang>,
}
impl_ast_node!(ItemFn, FnItem);

impl ItemFn {
    pub fn body(&self) -> Option<BlockExpr> {
        self.syntax.children().find_map(BlockExpr::cast)
    }
}
