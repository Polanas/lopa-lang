// use crate::parsing::ast::{self, LiteralKind, SyntaxNodePtr};
// use paste::paste;
// use ustr::Ustr;
//
// macro_rules! structs {
//     (
//         $(
//           $(#[$m:meta])*
//           $name: ident {
//
//               $($(#[$fm:meta])* $field_name:ident : $field_type:ty),* $(,)?
//           }
//         ),+ $(,)?
//     ) => {
//             $(
//                 $(#[$m])*
//                 #[derive(Clone, PartialEq, Eq, Hash, salsa::Update)]
//                 pub struct $name {
//                     pub node_ptr: SyntaxNodePtr,
//                     $(
//                         $(#[$fm])*
//                         pub $field_name: $field_type
//                     ),*
//                 }
//
//                 impl std::fmt::Debug for $name {
//                     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//                         f.debug_struct(stringify!($name))
//                             $(.field(stringify!($field_name), &self.$field_name))*
//                             .finish()
//                     }
//                 }
//
//                 impl WithNodePtr for $name {
//                     fn node_ptr(&self) -> SyntaxNodePtr {
//                         self.node_ptr
//                     }
//                 }
//             )+
//     };
// }
//
// macro_rules! enums {
//     (
//         $(
//             $name: ident {
//                 $(
//                     $variant: ident
//                 ),+ $(,)?
//             }
//         ),+ $(,)?
//     ) => {
//         paste! {
//             $(
//                 #[derive(Clone, PartialEq, Eq, Hash)]
//                 pub enum $name {
//                     $($variant($variant)),*
//                 }
//
//                 impl std::fmt::Debug for $name {
//                     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//                         match self {
//                             $(
//                                 #[allow(non_snake_case)]
//                                 $name::$variant([<_ $variant>]) => {
//                                     [<_ $variant>].fmt(f)
//                                 }
//                             ),*
//                         }
//                     }
//                 }
//
//                 impl WithNodePtr for $name {
//                     fn node_ptr(&self) -> SyntaxNodePtr {
//                         match self {
//                             $(
//                                 #[allow(non_snake_case)]
//                                 $name::$variant([<_ $variant>]) => {
//                                     [<_ $variant>].node_ptr()
//                                 }
//                             ),*
//                         }
//                     }
//                 }
//             )+
//         }
//     };
// }
//
// #[derive(Clone, Debug, PartialEq, Eq)]
// pub enum Primitive {
//     Int,
//     Float,
//     Bool,
//     String,
// }
//
// structs! {
//     // Missing {},
//     File {
//         items: Vec<Item>,
//     },
//     FnItem {
//         name: Name,
//         params: Vec<FnParam>,
//         output: Option<ReturnType>,
//     },
//     FnParam {
//         name: Name,
//         ty: TypeExpr,
//         default_value: Option<ast::Expr>,
//     },
//     ReturnType {
//         value: TypeExpr,
//     },
//     // ExprStmt {
//     //     expr: Expr,
//     // },
//     // LetStmt {
//     //     name: Name,
//     //     ty: TypeExpr,
//     //     expr: Expr,
//     // },
//     NilableType {
//         expr: Box<TypeExpr>,
//     },
//     AnyType {},
//     LitType {
//         kind: LiteralKind,
//     },
//     // NameExpr {
//     //     value: Name,
//     // },
//     // UnaryExpr {
//     //     expr: Box<Expr>,
//     //     kind: UnaryOpKind,
//     // },
//     // ReturnExpr {
//     //     expr: Box<Expr>,
//     // },
//     // IndexExpr {
//     //     base: Box<Expr>,
//     //     index: Box<Expr>,
//     // },
//     // Arg {
//     //     name: Option<Name>,
//     //     value: Expr,
//     // },
//     // CallExpr {
//     //     func: Box<Expr>,
//     //     args: Vec<Arg>,
//     // },
//     // ParenExpr {
//     //     expr: Box<Expr>,
//     // },
//     // BinaryExpr {
//     //     left: Box<Expr>,
//     //     right: Box<Expr>,
//     //     op: BinaryOpKind,
//     // },
//     // #[derive(Default)]
//     // BlockExpr {
//     //     stmts: Vec<Stmt>,
//     // },
//     // LitExpr {
//     //     kind: LiteralKind,
//     // },
//     Name {
//         value: Ustr,
//     },
// }
//
// enums! {
//     Item {
//         FnItem,
//     },
//     // Stmt {
//     //     LetStmt,
//     //     ExprStmt,
//     // },
//     // Expr {
//     //     Missing,
//     //     LitExpr,
//     //     BinaryExpr,
//     //     UnaryExpr,
//     //     BlockExpr,
//     //     IndexExpr,
//     //     CallExpr,
//     //     ParenExpr,
//     //     ReturnExpr,
//     //     NameExpr,
//     // }l;sdkj,
//     TypeExpr {
//         Name,
//         NilableType,
//         LitType,
//         AnyType,
//     },
// }
//
// pub trait WithNodePtr {
//     fn node_ptr(&self) -> SyntaxNodePtr;
// }
