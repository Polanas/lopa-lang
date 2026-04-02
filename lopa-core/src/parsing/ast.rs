use super::{lexer::Syntax, parser};
use rowan::ast::AstNode;

pub type SyntaxNode = rowan::SyntaxNode<parser::Lang>;

macro_rules! impl_ast_node {
    ($type:ident, $name:ident) => {
        impl AstNode for $type {
            type Language = parser::Lang;

            fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
            where
                Self: Sized,
            {
                kind == Syntax::$name
            }

            fn cast(syntax: SyntaxNode) -> Option<Self>
            where
                Self: Sized,
            {
                Self::can_cast(syntax.kind()).then(|| Self { syntax })
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.syntax
            }
        }
    };
}

macro_rules! def_ast_node {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            pub syntax: SyntaxNode,
        }
    };
}

macro_rules! def_impl_ast_node {
    ($type:ident, $name:ident) => {
        def_ast_node!($type);
        impl_ast_node!($type, $name);
    };
}

def_impl_ast_node!(FnItem, FN_ITEM);
def_impl_ast_node!(ParamList, PARAM_LIST);
def_impl_ast_node!(Param, PARAM);
impl FnItem {
    pub fn params(&self) -> impl Iterator<Item = Param> {
        self.syntax.children().filter_map(Param::cast)
    }
    pub fn param_list(&self) -> Option<ParamList> {
        self.syntax.children().find_map(ParamList::cast)
    }
    pub fn body(&self) -> Option<BlockExpr> {
        self.syntax.children().find_map(BlockExpr::cast)
    }
}

def_ast_node!(LiteralExpr);
def_ast_node!(ParenExpr);
def_ast_node!(CallExpr);
def_ast_node!(IndexExpr);
def_ast_node!(ReturnExpr);
def_impl_ast_node!(BlockExpr, BLOCK_EXPR);
def_ast_node!(BinaryExpr);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Literal(LiteralExpr),
    Paren(ParenExpr),
    Call(CallExpr),
    Index(IndexExpr),
    Return(ReturnExpr),
    Block(BlockExpr),
    Binary(BinaryExpr),
}

impl AstNode for Expr {
    type Language = parser::Lang;

    fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
    where
        Self: Sized,
    {
        matches!(
            kind,
            Syntax::LIT_EXPR
                | Syntax::PAREN_EXPR
                | Syntax::CALL_EXPR
                | Syntax::INDEX_EXPR
                | Syntax::RETURN_EXPR
                | Syntax::BLOCK_EXPR
                | Syntax::BINARY_EXPR
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self>
    where
        Self: Sized,
    {
        Some(match syntax.kind() {
            Syntax::LIT_EXPR => Self::Literal(LiteralExpr { syntax }),
            Syntax::PAREN_EXPR => Self::Paren(ParenExpr { syntax }),
            Syntax::CALL_EXPR => Self::Call(CallExpr { syntax }),
            Syntax::INDEX_EXPR => Self::Index(IndexExpr { syntax }),
            Syntax::RETURN_EXPR => Self::Return(ReturnExpr { syntax }),
            Syntax::BLOCK_EXPR => Self::Block(BlockExpr { syntax }),
            Syntax::BINARY_EXPR => Self::Binary(BinaryExpr { syntax }),
            _ => return None,
        })
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Expr::Literal(expr) => &expr.syntax,
            Expr::Paren(expr) => &expr.syntax,
            Expr::Call(expr) => &expr.syntax,
            Expr::Index(expr) => &expr.syntax,
            Expr::Return(expr) => &expr.syntax,
            Expr::Block(expr) => &expr.syntax,
            Expr::Binary(expr) => &expr.syntax,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Fn(FnItem),
}

impl AstNode for Item {
    type Language = parser::Lang;

    fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
    where
        Self: Sized,
    {
        matches!(kind, Syntax::FN_ITEM)
    }

    fn cast(syntax: SyntaxNode) -> Option<Self>
    where
        Self: Sized,
    {
        Some(match syntax.kind() {
            Syntax::FN_ITEM => Self::Fn(FnItem { syntax }),
            _ => return None,
        })
    }

    fn syntax(&self) -> &SyntaxNode {
        todo!()
    }
}

def_impl_ast_node!(File, FILE);
impl File {
    pub fn items(&self) -> impl Iterator<Item = Item> {
        self.syntax.children().filter_map(Item::cast)
    }
}
