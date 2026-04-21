pub mod lower;
pub mod module;

use crate::parsing::ast::{BinaryOpKind, LiteralKind, SyntaxNodePtr};
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
    Ident {
        value: String,
    },
    FnItem {
        name: Ident,
        params: Vec<FnParam>,
        output: ReturnType,
        body: BlockExpr,
    },
    FnParam {
        ident: Option<Ident>,
        ty: TypeExpr,
        default_value: Option<Expr>,
    },
    ReturnType {
        value: Option<TypeExpr>,
    },
    StmtExpr {
        expr: Expr,
    },
    NilableType {
        expr: Box<TypeExpr>,
    },
    AnyType {},
    LitType {
        kind: LiteralKind,
    },

    LitExpr {
        kind: LiteralKind,
    },
    BinaryExpr {
        left: Box<Expr>,
        right: Box<Expr>,
        op: BinaryOpKind,
    },
    BlockExpr {
        stmts: Vec<Stmt>,
    }
}

enums! {
    Item {
        FnItem,
    },
    TypeExpr {
        NilableType,
        AnyType,
        LitType,
    },

    Stmt {
        StmtExpr,
    },
    Expr {
        LitExpr,
        BinaryExpr,
        BlockExpr,
    }
}

pub trait WithNodePtr {
    fn node_ptr(&self) -> SyntaxNodePtr;
}
