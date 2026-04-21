use super::{lexer::Syntax, parser};
use crate::T;
use rowan::NodeOrToken;
use rowan::ast::support::{child, children, token};
use rowan::ast::{AstChildren, AstNode};

pub type SyntaxNode = rowan::SyntaxNode<parser::Lang>;
pub type SyntaxToken = rowan::SyntaxToken<parser::Lang>;
pub type SyntaxNodePtr = rowan::ast::SyntaxNodePtr<parser::Lang>;

//used for the matches! check in can_cast for enums
trait NodeWrapper {
    const KIND: Syntax;
}

macro_rules! structs {
    (
      $(
          $kind:ident = $name:ident $([$trait:tt])?
          { $($impl:tt)* } $(,)?
      ),*
    ) => {
        $(
            #[derive(Clone, Debug, PartialEq, Eq, Hash)]
            pub struct $name(pub SyntaxNode);

            impl $name {
                struct_impl!($($impl)*);
            }

            impl NodeWrapper for $name {
                const KIND: Syntax = Syntax::$kind;
            }

            $(impl $trait for $name{})*

            impl AstNode for $name {
                type Language = parser::Lang;

                fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
                where
                    Self: Sized,
                {
                    kind == Syntax::$kind
                }

                fn cast(syntax: SyntaxNode) -> Option<Self>
                where
                    Self: Sized,
                {
                    Self::can_cast(syntax.kind()).then(|| Self(syntax))
                }

                fn syntax(&self) -> &SyntaxNode {
                    &self.0
                }
            }
        )*
    };
}

macro_rules! struct_impl {
    () => {};
    //regular child, any
    ($field:ident: $ast:ident, $($tt:tt)*) => {
        pub fn $field(&self) -> Option<$ast> {
            child(&self.0)
        }
        struct_impl!($($tt)*);
    };
    //child with an offset
    ($field:ident[$k:tt]: $ast:ident, $($tt:tt)*) => {
        pub fn $field(&self) -> Option<$ast> { children(&self.0).nth($k) }
        struct_impl!($($tt)*);
    };
    //list of children
    ($field:ident: [$ast:ident], $($tt:tt)*) => {
        pub fn $field(&self) -> AstChildren<$ast> {
            children(&self.0)
        }
        struct_impl!($($tt)*);
    };
    //token
    ($field:ident: T![$tok:tt], $($tt:tt)*) => {
        pub fn $field(&self) -> Option<SyntaxToken> {
            token(&self.0, T![$tok])
        }
        struct_impl!($($tt)*);
    };
    ($($item:item)*) => {
        $($item)*
    }
}

macro_rules! enums {
    (
        $(
            $name:ident {
                $(
                    $variant:ident$(<$generic:ident>)?
                ),* $(,)?
            }
        ),* $(,)?
    ) => {
        $(
            #[derive(Clone, Debug, PartialEq, Eq, Hash)]
            pub enum $name {
                $($variant($variant$(<$generic>)*),)*
            }

            impl AstNode for $name {
                type Language = parser::Lang;

                fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
                where
                    Self: Sized,
                {
                    matches!(kind, $(<$variant as NodeWrapper>::KIND)|*)
                }

                fn cast(syntax: SyntaxNode) -> Option<Self>
                where
                    Self: Sized,
                {
                    match syntax.kind() {
                        $(<$variant as NodeWrapper>::KIND => Some(Self::$variant$(::<$generic>)*($variant(syntax))),)*
                        _ => None,
                    }
                }

                fn syntax(&self) -> &SyntaxNode {
                    match self {
                        $(Self::$variant(e) => &e.0,)*
                    }
                }
            }
        )*
    };
}
structs! {
    FILE = File {
        items: [Item],
    },
    FN_ITEM = FnItem {

    },
    EXPR_STMT = ExprStmt {
        expr: Expr,
    },
    LET_STMT = LetStmt {
        ident: Ident,
        ty: TypeExpr,
        expr: Expr,

        eq_token: T![=],
    },
    UNARY_EXPR = UnaryExpr {
        expr: Expr,

        pub fn op_token(&self) -> Option<SyntaxToken> {
            self.op_details().map(|t| t.0)
        }

        pub fn op_kind(&self) -> Option<UnaryOpKind> {
            self.op_details().map(|t| t.1)
        }

        pub fn op_details(&self) -> Option<(SyntaxToken, UnaryOpKind)> {
            self.syntax().children_with_tokens().find_map(|c| {
                let token = c.into_token()?;
                let op = match token.kind() {
                    T![-] => UnaryOpKind::Neg,
                    T![!] => UnaryOpKind::Not,
                    _ => return None,
                };
                Some((token, op))
            })
        }
    },
    BINARY_EXPR = BinaryExpr {
        lhs: Expr,
        rhs[1]: Expr,

        pub fn op_token(&self) -> Option<SyntaxToken> {
            self.op_details().map(|t| t.0)
        }

        pub fn op_kind(&self) -> Option<BinaryOpKind> {
            self.op_details().map(|t| t.1)
        }

        pub fn op_details(&self) -> Option<(SyntaxToken, BinaryOpKind)> {
            self.syntax().children_with_tokens().find_map(|c| {
                let token = c.into_token()?;
                let op = match token.kind() {
                    T![+] => BinaryOpKind::Add,
                    T![*] => BinaryOpKind::Mul,
                    T![/] => BinaryOpKind::Div,
                    T!["//"] => BinaryOpKind::DivInt,
                    T![%] => BinaryOpKind::Rem,
                    T![or] => BinaryOpKind::Or,
                    // T![] => BinaryOpKind::Shl,
                    // T![] => BinaryOpKind::Shr,
                    // T![] => BinaryOpKind::BitXor,
                    // T![] => BinaryOpKind::BitAnd,
                    T![-] => BinaryOpKind::Sub,
                    T![>] => BinaryOpKind::Greater,
                    T![>=] => BinaryOpKind::GreaterEqual,
                    T![<] => BinaryOpKind::Less,
                    T![<=] => BinaryOpKind::LessEqual,
                    T![!=] => BinaryOpKind::NotEqual,
                    T![==] => BinaryOpKind::Equal,
                    T![and] => BinaryOpKind::And,
                    T![|] => BinaryOpKind::BitOr,
                    T![+=] => BinaryOpKind::AddAssign,
                    T![-=] => BinaryOpKind::SubAssign,
                    T![*=] => BinaryOpKind::MulAssign,
                    T![/=] => BinaryOpKind::DivAssign,
                    T!["//="] => BinaryOpKind::DivIntAssign,
                    T![%=] => BinaryOpKind::RemAssign,
                    // T![] => BinaryOpKind::BitXorAssign,
                    // T![] => BinaryOpKind::BitAndAssign,
                    // T![] => BinaryOpKind::BitOrAssign,
                    // T![] => BinaryOpKind::ShlAssign,
                    // T![] => BinaryOpKind::ShrAssign,
                    _ => return None,
                };
                Some((token, op))
            })
        }
    },
    CALL_EXPR = CallExpr {

    },
    PAREN_EXPR = ParenExpr {
        expr: Expr,
    },
    BLOCK_EXPR = Block {
        exprs: [ExprStmt],
    },
    LIT_EXPR = LiteralExpr {
        pub fn token(&self) -> Option<SyntaxToken> {
            self.0.children_with_tokens().find_map(NodeOrToken::into_token)
        }

        pub fn kind(&self) -> Option<LiteralKind> {
            Some(match self.token()?.kind() {
                Syntax::INT => LiteralKind::Int,
                Syntax::FLOAT => LiteralKind::Float,
                Syntax::STRING => LiteralKind::String,
                _ => return None,
            })
        }
    },
    IDENT = Ident {
        pub fn token(&self) -> Option<SyntaxToken> {
            self.0.children_with_tokens().find_map(NodeOrToken::into_token)
        }
    },
}

enums! {
    Item {
        FnItem,
    },
    Stmt {
        LetStmt,
        ExprStmt,
    },
    Expr {
        LiteralExpr,
    },
    TypeExpr {
        Ident,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralKind {
    Int,
    Float,
    String,
    Bool,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum BinaryOpKind {
    Add,
    Mul,
    Div,
    DivInt,
    Rem,
    Or,
    Shl,
    Shr,
    BitXor,
    BitAnd,
    Sub,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
    Equal,
    And,
    BitOr,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    DivIntAssign,
    RemAssign,
    BitXorAssign,
    BitAndAssign,
    BitOrAssign,
    ShlAssign,
    ShrAssign,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum UnaryOpKind {
    Not,
    Neg,
}

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

// macro_rules! def_ast_node {
//     ($name:ident) => {
//         #[derive(Debug, Clone, PartialEq, Eq)]
//         pub struct $name {
//             pub syntax: SyntaxNode,
//         }
//     };
// }
//
// macro_rules! def_impl_ast_node {
//     ($type:ident, $name:ident) => {
//         def_ast_node!($type);
//         impl_ast_node!($type, $name);
//     };
// }
//
// def_impl_ast_node!(FnItem, FN_ITEM);
// def_impl_ast_node!(ParamList, PARAM_LIST);
// def_impl_ast_node!(Param, PARAM);
// impl FnItem {
//     pub fn params(&self) -> impl Iterator<Item = Param> {
//         self.syntax.children().filter_map(Param::cast)
//     }
//     pub fn param_list(&self) -> Option<ParamList> {
//         self.syntax.children().find_map(ParamList::cast)
//     }
//     pub fn body(&self) -> Option<BlockExpr> {
//         self.syntax.children().find_map(BlockExpr::cast)
//     }
// }
//
// def_ast_node!(LiteralExpr);
// def_ast_node!(ParenExpr);
// def_ast_node!(CallExpr);
// def_ast_node!(IndexExpr);
// def_ast_node!(ReturnExpr);
// def_impl_ast_node!(BlockExpr, BLOCK_EXPR);
// def_ast_node!(BinaryExpr);
//
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum Expr {
//     Literal(LiteralExpr),
//     Paren(ParenExpr),
//     Call(CallExpr),
//     Index(IndexExpr),
//     Return(ReturnExpr),
//     Block(BlockExpr),
//     Binary(BinaryExpr),
// }
//
// impl AstNode for Expr {
//     type Language = parser::Lang;
//
//     fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
//     where
//         Self: Sized,
//     {
//         matches!(
//             kind,
//             Syntax::LIT_EXPR
//                 | Syntax::PAREN_EXPR
//                 | Syntax::CALL_EXPR
//                 | Syntax::INDEX_EXPR
//                 | Syntax::RETURN_EXPR
//                 | Syntax::BLOCK_EXPR
//                 | Syntax::BINARY_EXPR
//         )
//     }
//
//     fn cast(syntax: SyntaxNode) -> Option<Self>
//     where
//         Self: Sized,
//     {
//         Some(match syntax.kind() {
//             Syntax::LIT_EXPR => Self::Literal(LiteralExpr { syntax }),
//             Syntax::PAREN_EXPR => Self::Paren(ParenExpr { syntax }),
//             Syntax::CALL_EXPR => Self::Call(CallExpr { syntax }),
//             Syntax::INDEX_EXPR => Self::Index(IndexExpr { syntax }),
//             Syntax::RETURN_EXPR => Self::Return(ReturnExpr { syntax }),
//             Syntax::BLOCK_EXPR => Self::Block(BlockExpr { syntax }),
//             Syntax::BINARY_EXPR => Self::Binary(BinaryExpr { syntax }),
//             _ => return None,
//         })
//     }
//
//     fn syntax(&self) -> &SyntaxNode {
//         match self {
//             Expr::Literal(expr) => &expr.syntax,
//             Expr::Paren(expr) => &expr.syntax,
//             Expr::Call(expr) => &expr.syntax,
//             Expr::Index(expr) => &expr.syntax,
//             Expr::Return(expr) => &expr.syntax,
//             Expr::Block(expr) => &expr.syntax,
//             Expr::Binary(expr) => &expr.syntax,
//         }
//     }
// }
//
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum Item {
//     Fn(FnItem),
// }
//
// impl AstNode for Item {
//     type Language = parser::Lang;
//
//     fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
//     where
//         Self: Sized,
//     {
//         matches!(kind, Syntax::FN_ITEM)
//     }
//
//     fn cast(syntax: SyntaxNode) -> Option<Self>
//     where
//         Self: Sized,
//     {
//         Some(match syntax.kind() {
//             Syntax::FN_ITEM => Self::Fn(FnItem { syntax }),
//             _ => return None,
//         })
//     }
//
//     fn syntax(&self) -> &SyntaxNode {
//         todo!()
//     }
// }
//
// def_impl_ast_node!(File, FILE);
// impl File {
//     pub fn items(&self) -> impl Iterator<Item = Item> {
//         self.syntax.children().filter_map(Item::cast)
//     }
// }
