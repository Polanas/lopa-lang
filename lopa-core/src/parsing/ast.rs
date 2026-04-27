use super::{lexer::Syntax, parser};
use crate::T;
use rowan::NodeOrToken;
use rowan::ast::support::{child, children, token};
use rowan::ast::{AstChildren, AstNode};
use ustr::Ustr;

pub type SyntaxNode = rowan::SyntaxNode<parser::Lang>;
pub type SyntaxToken = rowan::SyntaxToken<parser::Lang>;
pub type SyntaxNodePtr = rowan::ast::SyntaxNodePtr<parser::Lang>;
pub type AstPtr<N: rowan::ast::AstNode> = rowan::ast::AstPtr<N>;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralKind {
    Int,
    Float,
    String,
    Bool,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
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

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum UnaryOpKind {
    Not,
    Neg,
}

structs! {
    FILE = File {
        items: [Item],
    },
    FN_ITEM = FnItem {
        fn_token: T![fn],
        name: Name,
        params: ParamList,
        output: ReturnType,
        body: BlockExpr,
    },
    RETURN_TYPE = ReturnType {
        arrow_token: T![->],
        ty: TypeExpr,
    },
    PARAM_LIST = ParamList {
        params: [Param],
    },
    PARAM = Param {
        name: Name,
        colon_token: T![:],
        ty: TypeExpr,
        default_value: Expr,
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
    NILABLE_TYPE = NilableType {
        mark_token: T![?],
        ty: TypeExpr,
    },
    ANY_TYPE = AnyType {
        ty: TypeExpr,
    },
    LIT_TYPE = LitType {
        ty: TypeExpr,
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
    RETURN_EXPR = ReturnExpr {
        expr: Expr,
    },
    INDEX_EXPR = IndexExpr {
        base: Expr,
        index[1]: Expr,
    },
    ARG_LIST = ArgList {
        args: [Arg],
    },
    ARG = Arg {
        name: Name,
        colon_token: T![:],
        value: Expr,
    },
    CALL_EXPR = CallExpr {
        func: Expr,
        args: ArgList,
    },
    PAREN_EXPR = ParenExpr {
        expr: Expr,
    },
    BLOCK_EXPR = BlockExpr {
        exprs: [Stmt],
    },
    LIT_EXPR = LitExpr {
        pub fn token(&self) -> Option<SyntaxToken> {
            self.0.children_with_tokens().find_map(NodeOrToken::into_token)
        }

        pub fn text(&self) -> Option<Ustr> {
            self.token().map(|t| t.text().into())
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
    NAME = Name {
        ident: Ident,

        pub fn text(&self) -> Option<Ustr> {
            self.ident().and_then(|i| i.text())
        }
    },
    IDENT = Ident {
        pub fn token(&self) -> Option<SyntaxToken> {
            self.0.children_with_tokens().find_map(NodeOrToken::into_token)
        }

        pub fn text(&self) -> Option<Ustr> {
            self.token().map(|t| t.text().into())
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
        LitExpr,
        BinaryExpr,
        UnaryExpr,
        BlockExpr,
        IndexExpr,
        CallExpr,
        ParenExpr,
    },
    TypeExpr {
        Ident,
        NilableType,
        LitType,
        AnyType,
    },
}
