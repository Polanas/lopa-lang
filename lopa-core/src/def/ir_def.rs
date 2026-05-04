use crate::parsing::ast::{BinaryOpKind, LiteralKind, SyntaxNodePtr, UnaryOpKind};
use paste::paste;
use ustr::Ustr;

macro_rules! structs {
    (
        $(
          $(#[$m:meta])*
          $name: ident {
              $($field_name:ident : $field_type:ty),* $(,)?
          }
        ),+ $(,)?
    ) => {
        $(
            $(#[$m])*
            #[derive(Clone, Debug, PartialEq, Eq, Hash)]
            pub struct $name {
                pub node_ptr: Option<SyntaxNodePtr>,
                $(
                    pub $field_name: $field_type
                ),*
            }

            impl WithNodePtr for $name {
                fn node_ptr(&self) -> Option<SyntaxNodePtr> {
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
            #[derive(Clone, Debug, PartialEq, Eq, Hash)]
            pub enum $name {
                $($variant($variant)),*
            }

            impl WithNodePtr for $name {
                fn node_ptr(&self) -> Option<SyntaxNodePtr > {
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
    #[derive(Default)]
    Missing {},
    File {
        items: Vec<Item>,
    },
    FnItem {
        name: Name,
        params: Vec<FnParam>,
        output: Option<ReturnType>,
        body: BlockExpr,
    },
    FnParam {
        name: Name,
        ty: TypeExpr,
        default_value: Option<Expr>,
    },
    ReturnType {
        value: TypeExpr,
    },
    ExprStmt {
        expr: Expr,
    },
    LetStmt {
        name: Name,
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
    NameExpr {
        value: Name,
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
    #[derive(Default)]
    BlockExpr {
        stmts: Vec<Stmt>,
    },
    LitExpr {
        kind: LiteralKind,
    },
    Name {
        value: Ustr,
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
        Missing,
        LitExpr,
        BinaryExpr,
        UnaryExpr,
        BlockExpr,
        IndexExpr,
        CallExpr,
        ParenExpr,
        ReturnExpr,
        NameExpr,
    },
    TypeExpr {
        Name,
        NilableType,
        LitType,
        AnyType,
    },
}

pub trait WithNodePtr {
    fn node_ptr(&self) -> Option<SyntaxNodePtr>;
}
