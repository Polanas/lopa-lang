use std::ops::Deref;

use la_arena::{Idx, RawIdx};
use notify_rust::Notification;
use rowan::ast::AstNode;
use salsa::Accumulator;
use ustr::Ustr;

use crate::{
    common::LitKind,
    def::lower::{lower_item_type_expr_with_self, lower_type_expr, lower_type_expr_with_self},
    ide::{
        self,
        diagnostics::{Diagnostic, DiagnosticKind},
    },
    parsing::ast::{self, BinaryOpKind, UnaryOpKind},
    ustr_hash::{UstrHash, UstrIndexMap},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, salsa::Update)]
pub struct Local<'db> {
    pub parent: Function<'db>,
    pub pattern_id: PatternId,
}

#[derive(salsa::Supertype, Clone, PartialEq, Eq, Hash, Debug, salsa::Update)]
pub enum ModuleDef<'db> {
    Function(Function<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    Module(ide::File),
}

impl ModuleDef<'_> {
    fn kind(&self) -> ModuleDefKind {
        match self {
            ModuleDef::Function(_) => ModuleDefKind::Function,
            ModuleDef::Struct(_) => ModuleDefKind::Struct,
            ModuleDef::Module(_) => ModuleDefKind::Module,
            ModuleDef::Enum(_) => ModuleDefKind::Enum,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, salsa::Update)]
pub enum ModuleDefKind {
    Function,
    Struct,
    Enum,
    Module,
}

#[salsa::tracked(debug)]
pub struct Module<'db> {
    pub file: ide::File,
    pub ast_ptr: ast::AstPtr<ast::ModItem>,
}

#[derive(salsa::Supertype, salsa::Update, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ImplItem<'db> {
    Function(Function<'db>),
}

// #[salsa::tracked]
// impl<'db> ImplFunction<'db> {
//     #[salsa::tracked(returns(ref))]
//     fn is_method(self, db: &'db dyn salsa::Database) -> bool {
//         self.params(db)
//             .iter()
//             .next()
//             .map(|p| p.ty == self.owner(db))
//             .unwrap_or_default()
//     }
//
//     #[salsa::tracked(returns(ref))]
//     pub fn params(self, db: &'db dyn salsa::Database) -> Vec<Param<'db>> {
//         let mut params = vec![];
//         let file = self.func(db).file(db);
//         let root = ide::parse(db, file).syntax_node(db);
//         for param in self
//             .func(db)
//             .ast_ptr(db)
//             .to_node(&root)
//             .params()
//             .into_iter()
//             .flat_map(|p| p.params())
//         {
//             //TODO: check for self
//             let name = param.pattern().and_then(|p| {
//                 Some(match p {
//                     ast::Pattern::NamePattern(name_patern) => name_patern,
//                 })
//                 .and_then(|n| n.name())
//                 .and_then(|n| n.text())
//             });
//             let ty = param
//                 .ty()
//                 .map(|ty| lower_type_expr_with_self(db, file, ty, Some(self.owner(db))))
//                 .unwrap_or_else(|| Type::Unknown);
//             params.push(Param { name, ty });
//         }
//         params
//     }
//
//     #[salsa::tracked(returns(ref))]
//     pub fn output(self, db: &'db dyn salsa::Database) -> Type<'db> {
//         let file = self.func(db).file(db);
//         let root = ide::parse(db, file).syntax_node(db);
//         let output = self
//             .func(db)
//             .ast_ptr(db)
//             .to_node(&root)
//             .output()
//             .and_then(|o| o.ty());
//         output
//             .map(|o| lower_type_expr_with_self(db, file, o, Some(self.owner(db))))
//             .unwrap_or_else(|| Type::Unit)
//     }
// }

#[salsa::tracked(debug)]
pub struct Function<'db> {
    pub name: Ustr,
    pub ast_ptr: ast::AstPtr<ast::FnItem>,
    pub file: ide::File,
    pub owner: Option<Type<'db>>,
    pub implementee: Option<Type<'db>>,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub struct Param<'db> {
    pub name: Option<Ustr>,
    pub ty: Type<'db>,
}
#[salsa::tracked(returns(ref))]
pub fn function_params<'db>(
    db: &'db dyn salsa::Database,
    function: Function<'db>,
) -> Vec<Param<'db>> {
    let mut params = vec![];
    let file = function.file(db);
    let root = ide::parse(db, file).syntax_node(db);
    for param in function
        .ast_ptr(db)
        .to_node(&root)
        .params()
        .into_iter()
        .flat_map(|p| p.params())
    {
        if param.self_token().is_some() {
            let Some(owner) = function.owner(db) else {
                Diagnostic::new(
                    param.syntax().text_range(),
                    DiagnosticKind::TypeError,
                    "`self` parameter is only allowed in associated functions".to_string(),
                )
                .accumulate(db);
                continue;
            };
            params.push(Param {
                name: Some("self".into()),
                ty: owner,
            });
        } else {
            let name = param.pattern().and_then(|p| {
                match p {
                    ast::Pattern::NamePattern(name_patern) => Some(name_patern),
                    _ => None,
                }
                .and_then(|n| n.name())
                .and_then(|n| n.text())
            });
            let ty = param
                .ty()
                .map(|ty| lower_type_expr_with_self(db, file, ty, function.owner(db)))
                .unwrap_or_else(|| Type::Unknown);
            params.push(Param { name, ty });
        }
    }
    params
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    fn is_method(self, db: &'db dyn salsa::Database) -> bool {
        function_params(db, self)
            .iter()
            .next()
            .and_then(|p| self.owner(db).map(|o| p.ty == o))
            .unwrap_or_default()
    }

    #[salsa::tracked(returns(ref))]
    pub fn params_by_name(self, db: &'db dyn salsa::Database) -> UstrIndexMap<Param<'db>> {
        function_params(db, self)
            .iter()
            .filter_map(|p| p.name.map(|n| (UstrHash(n), p.clone())))
            .collect()
    }

    #[salsa::tracked(returns(ref))]
    pub fn bare_fn_ty(self, db: &'db dyn salsa::Database) -> BareFn<'db> {
        BareFn {
            params: function_params(db, self).clone(),
            output: self.output(db).clone().into(),
        }
    }

    #[salsa::tracked(returns(ref))]
    pub fn output(self, db: &'db dyn salsa::Database) -> Type<'db> {
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        let output = self
            .ast_ptr(db)
            .to_node(&root)
            .output()
            .and_then(|o| o.ty());
        output
            .map(|o| lower_type_expr(db, file, o))
            .unwrap_or_else(|| Type::Unit)
    }
}

#[salsa::tracked(debug)]
pub struct UseItem<'db> {
    pub ast_ptr: ast::AstPtr<ast::UseItem>,
}

#[salsa::tracked(debug)]
pub struct Enum<'db> {
    pub name: Ustr,
    pub ast_ptr: ast::AstPtr<ast::EnumItem>,
    pub file: ide::File,
}

#[salsa::tracked(debug)]
pub struct Struct<'db> {
    pub name: Ustr,
    pub ast_ptr: ast::AstPtr<ast::StructItem>,
    pub file: ide::File,
}

#[derive(salsa::Supertype, salsa::Update, PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum StructElem<'db> {
    Field(Field<'db>),
    Fn(Function<'db>),
}

#[salsa::tracked(debug)]
pub struct Field<'db> {
    pub name: Ustr,
    pub ty: Type<'db>,
    pub ast_ptr: ast::AstPtr<ast::Field>,
}

// #[salsa::tracked(returns(ref))]
// pub fn struct_non_static_methods<'db>(
//     db: &'db dyn salsa::Database,
//     struct_item: Struct<'db>,
//     files: Files,
// ) -> UstrIndexMap<ir::ImplFunction<'db>> {
//     // ide::impls(db, files)
//     //     .functions
//     //     .get(&ImplPair {
//     //         implementee: Type::Struct(struct_item),
//     //         impl_ty: None,
//     //     })
//     //     .map(|fns| {
//     //         fns.iter()
//     //             .filter(|f| *f.is_method(db))
//     //             .map(|f| (UstrHash(f.func(db).name(db)), *f))
//     //             .collect::<UstrIndexMap<ir::ImplFunction<'db>>>()
//     //     })
//     //     .unwrap_or_default()
// }

// #[salsa::tracked(returns(ref))]
// pub fn struct_fns_for_ty<'db>(
//     db: &'db dyn salsa::Database,
//     struct_item: Struct<'db>,
//     impl_ty: Type<'db>,
//     files: Files,
// ) -> UstrIndexMap<ir::Function<'db>> {
//     ide::impls(db, files)
//         .functions
//         .get(&ImplPair {
//             implementee: Type::Struct(struct_item),
//             impl_ty: Some(impl_ty),
//         })
//         .map(|fns| {
//             fns.iter()
//                 .map(|f| (UstrHash(f.name(db)), *f))
//                 .collect::<UstrIndexMap<ir::ImplFunction<'db>>>()
//         })
//         .unwrap_or_default()
// }
#[salsa::tracked(returns(ref))]
pub fn enum_fields<'db>(db: &'db dyn salsa::Database, enum_item: Enum<'db>) -> Vec<Field<'db>> {
    let mut fields = vec![];
    let file = enum_item.file(db);
    let root = ide::parse(db, file).syntax_node(db);
    for element in enum_item.ast_ptr(db).to_node(&root).elements() {
        if let ast::EnumElem::Field(field) = element {
            let Some(name) = field.name().and_then(|n| n.text()) else {
                continue;
            };

            let ptr = ast::AstPtr::new(&field);
            let Some((ast_ty, ir_ty)) = field.ty().map(|ty| {
                (
                    ty.clone(),
                    lower_item_type_expr_with_self(db, file, ty, Some(Type::Enum(enum_item))),
                )
            }) else {
                continue;
            };

            fields.push(Field::new(db, name, ir_ty, ptr));
        }
    }

    fields
}

#[salsa::tracked(returns(ref))]
pub fn struct_impl_item<'db>(
    db: &'db dyn salsa::Database,
    struct_item: Struct<'db>,
    implementee: ide::Implementee<'db>,
    name: Ustr,
) -> Option<ImplItem<'db>> {
    let impl_map = ide::impl_map(db, struct_item.file(db).source_root(db));
    let key = ide::ImplKey {
        implementor: Type::Struct(struct_item),
        implementee,
    };
    impl_map.get(&key)?.get(&name).cloned()
}

#[salsa::tracked(returns(ref))]
pub fn struct_fields<'db>(
    db: &'db dyn salsa::Database,
    struct_item: Struct<'db>,
) -> Vec<Field<'db>> {
    let mut fields = vec![];
    let file = struct_item.file(db);
    let root = ide::parse(db, file).syntax_node(db);
    for element in struct_item.ast_ptr(db).to_node(&root).elements() {
        if let ast::StructElem::Field(field) = element {
            let Some(name) = field.name().and_then(|n| n.text()) else {
                continue;
            };

            let ptr = ast::AstPtr::new(&field);
            let Some((ast_ty, ir_ty)) = field.ty().map(|ty| {
                (
                    ty.clone(),
                    lower_item_type_expr_with_self(db, file, ty, Some(Type::Struct(struct_item))),
                )
            }) else {
                continue;
            };

            fields.push(Field::new(db, name, ir_ty, ptr));
        }
    }

    fields
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub struct BareFn<'db> {
    pub params: Vec<Param<'db>>,
    pub output: Box<Type<'db>>,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum Type<'db> {
    Unknown,
    Any,
    Unit,
    Never,
    Lit(LitKind),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    Dyn(Struct<'db>),
    Function(Function<'db>),
    Nilable(Box<Type<'db>>),
    BareFn(BareFn<'db>),
}

impl<'db> Type<'db> {
    pub fn is_unknown(&self) -> bool {
        match self {
            Self::Unknown => true,
            Self::Nilable(nilable) if matches!(**nilable, Self::Unknown) => true,
            _ => false,
        }
    }
    pub fn is_nilable(&self) -> bool {
        matches!(
            self,
            Self::Nilable(_) | Self::Lit(LitKind::Nil) | Self::Unit
        )
    }
    fn collapse_nil_inner(&mut self) {
        if let Self::Nilable(inner) = self {
            inner.collapse_nil();

            if inner.is_nilable()
                && let Self::Nilable(deep_inner) = std::mem::replace(&mut **inner, Self::Any)
            {
                *self = Self::Nilable(deep_inner)
            }
        }
    }

    pub fn int() -> Self {
        Self::Lit(LitKind::Int)
    }

    pub fn float() -> Self {
        Self::Lit(LitKind::Float)
    }

    pub fn bool() -> Self {
        Self::Lit(LitKind::Bool)
    }

    pub fn any() -> Self {
        Self::Any
    }

    pub fn unit() -> Self {
        Self::Unit
    }

    pub fn never() -> Self {
        Self::Never
    }

    pub fn is_number(&self) -> bool {
        self.is_int() || self.is_float()
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Self::Lit(LitKind::Float))
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Self::Lit(LitKind::Int))
    }

    pub fn collapse_nil(&mut self) {
        self.collapse_nil_inner();
        if let Self::Nilable(inner) = self
            && **inner == Self::Lit(LitKind::Nil)
        {
            *self = Self::Lit(LitKind::Nil);
        }
    }

    pub fn collapsed_nil(mut self) -> Self {
        self.collapse_nil_inner();
        if let Self::Nilable(inner) = &self
            && inner.deref() == &Self::Lit(LitKind::Nil)
        {
            self = Self::Lit(LitKind::Nil);
        }
        self
    }
}

pub type ExprId = Idx<Expr>;

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash, Default)]
pub struct Path(pub Vec<Ustr>);

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub enum Expr {
    Missing,
    Unit,
    Path(Path),
    Lit(LitKind),
    As {
        expr: ExprId,
        ty: Type<'static>,
    },
    Is {
        expr: ExprId,
        pat: PatternId,
    },
    IsNot {
        expr: ExprId,
        pat: PatternId,
    },
    BlockExpr {
        stmts: Vec<StmtId>,
    },
    If {
        if_cond: ExprId,
        if_branch: ExprId,
        else_branch: Option<ExprId>,
    },
    Unary {
        expr: ExprId,
        kind: UnaryOpKind,
    },
    Binary {
        left: ExprId,
        right: ExprId,
        kind: BinaryOpKind,
    },
    Return {
        expr: ExprId,
    },
    Index {
        base: ExprId,
        index: ExprId,
    },
    Call {
        func: ExprId,
        args: Vec<Arg>,
    },
    Paren {
        expr: ExprId,
    },
    Field {
        name: Ustr,
        expr: ExprId,
    },
    Method {
        name: Ustr,
        expr: ExprId,
        args: Vec<Arg>,
    },
    Record {
        path: Vec<Ustr>,
        fields: Vec<RecordField>,
    },
    Closure {
        params: Vec<ClosureParam>,
        output: Type<'static>,
    },
    SelfVar,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub struct ClosureParam {
    pattern: PatternId,
    ty: Option<Type<'static>>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub struct RecordField {
    pub name: Ustr,
    pub expr: ExprId,
}

pub type PatternId = Idx<Pattern>;

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub enum Pattern {
    Missing,
    Wildcard,
    Path(Path),
    Name(Ustr),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Arg {
    Labeled { label: Ustr, value: ExprId },
    NonLabeled { value: ExprId },
}

impl Arg {
    pub fn value(&self) -> ExprId {
        match self {
            Arg::Labeled { value, .. } | Arg::NonLabeled { value } => *value,
        }
    }
}

pub type StmtId = RawIdx;

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub enum Stmt<'db> {
    Let {
        pat: PatternId,
        ty: Option<Type<'db>>,
        expr: ExprId,
    },
    Expr {
        expr: ExprId,
        semi: Option<()>,
    },
}
