pub mod lower;
pub mod module;

use crate::parsing::ast::{BinaryOpKind, LiteralKind, SyntaxNodePtr, UnaryOpKind};
use paste::paste;

macro_rules! structs {
    (
        $(
          $name: ident {
              $($field_name:ident : $field_type:ty),* $(,)?
          }
        ),+ $(,)?
    ) => {
        $(

            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct $name {
                node_ptr: SyntaxNodePtr,
                $(
                    pub $field_name: $field_type
                ),*
            }

            impl WithNodePtr for $name {
                fn node_ptr(&self) -> SyntaxNodePtr {
                    self.node_ptr
                }
            }
        )+
    };
}

macro_rules! enums {
    (
        $(
            $name: ident {
                $(
                    $variant: ident
                ),+ $(,)?
            }
        ),+ $(,)?
    ) => {
        $(
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub enum $name {
                $($variant($variant)),*
            }

            impl WithNodePtr for $name {
                fn node_ptr(&self) -> SyntaxNodePtr {
                    match self {
                        $(
                            #[allow(non_snake_case)]
                            $name::$variant(paste!{[<_ $variant>]}) => {
                                paste!{[<_ $variant>]}.node_ptr()
                            }
                        ),*
                    }
                }
            }
        )+
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Primitive {
    Int,
    Float,
    Bool,
    String,
}

structs! {
    File {
        items: Vec<Item>,
    },
    FnItem {
        name: Name,
        params: Vec<FnParam>,
        output: ReturnType,
        body: BlockExpr,
    },
    FnParam {
        name: Option<Ident>,
        ty: TypeExpr,
        default_value: Option<Expr>,
    },
    ReturnType {
        value: Option<TypeExpr>,
    },
    ExprStmt {
        expr: Expr,
    },
    LetStmt {
        ident: Ident,
        ty: TypeExpr,
        expr: Expr,
    },
    NilableType {
        expr: Box<TypeExpr>,
    },
    AnyType {},
    LitType {
        kind: LiteralKind,
    },
    UnaryExpr {
        expr: Box<Expr>,
        kind: UnaryOpKind,
    },
    ReturnExpr {
        expr: Box<Expr>,
    },
    IndexExpr {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Arg {
        name: Option<Name>,
        value: Expr,
    },
    CallExpr {
        func: Box<Expr>,
        args: Vec<Arg>,
    },
    ParenExpr {
        expr: Box<Expr>,
    },
    BinaryExpr {
        left: Box<Expr>,
        right: Box<Expr>,
        op: BinaryOpKind,
    },
    BlockExpr {
        stmts: Vec<Stmt>,
    },
    LitExpr {
        kind: LiteralKind,
    },
    Name {
        value: Ident,
    },
    Ident {
        value: String,
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

pub trait WithNodePtr {
    fn node_ptr(&self) -> SyntaxNodePtr;
}
