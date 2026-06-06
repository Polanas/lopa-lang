use std::ops::Deref;

use la_arena::{Idx, RawIdx};
use rowan::ast::{AstNode, AstPtr};
use salsa::Accumulator;
use ustr::{Ustr, UstrMap};

use crate::{
    common::LitKind,
    def::{
        ir,
        lower::{self, lower_type_expr, lower_type_expr_with_self},
        scope,
    },
    ide::{
        self,
        diagnostics::{Diagnostic, DiagnosticKind},
    },
    parsing::ast::{self, BinaryOpKind, UnaryOpKind},
    ustr_hash::{UstrHash, UstrIndexMap},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Local<'db> {
    pub parent: Function<'db>,
    pub pattern_id: PatternId,
}

#[derive(salsa::Supertype, Clone, PartialEq, Eq, Hash, Debug, salsa::Update)]
pub enum ModuleValueDef<'db> {
    Function(Function<'db>),
}

#[derive(salsa::Supertype, Clone, PartialEq, Eq, Hash, Debug, salsa::Update)]
pub enum ModuleTypeDef<'db> {
    Struct(Struct<'db>),
    Module(ide::File),
}

#[salsa::tracked(debug)]
pub struct ImplFunction<'db> {
    pub func: Function<'db>,
    pub owner: Type<'db>,
}

#[salsa::tracked]
impl<'db> ImplFunction<'db> {
    #[salsa::tracked(returns(ref))]
    fn is_method(self, db: &'db dyn salsa::Database) -> bool {
        self.params(db)
            .iter()
            .next()
            .map(|p| p.ty == self.owner(db))
            .unwrap_or_default()
    }

    #[salsa::tracked(returns(ref))]
    pub fn params(self, db: &'db dyn salsa::Database) -> Vec<Param<'db>> {
        let mut params = vec![];
        let file = self.func(db).file(db);
        let root = ide::parse(db, file).syntax_node(db);
        for param in self
            .func(db)
            .ast_ptr(db)
            .to_node(&root)
            .params()
            .into_iter()
            .flat_map(|p| p.params())
        {
            //TODO: check for self
            let name = param.pattern().and_then(|p| {
                Some(match p {
                    ast::Pattern::NamePattern(name_patern) => name_patern,
                })
                .and_then(|n| n.name())
                .and_then(|n| n.text())
            });
            let ty = param
                .ty()
                .map(|ty| lower_type_expr_with_self(db, file, ty, Some(self.owner(db))))
                .unwrap_or_else(|| Type::Unknown(Ustr::from("")));
            params.push(Param { name, ty });
        }
        params
    }

    #[salsa::tracked(returns(ref))]
    pub fn output(self, db: &'db dyn salsa::Database) -> Type<'db> {
        let file = self.func(db).file(db);
        let root = ide::parse(db, file).syntax_node(db);
        let output = self
            .func(db)
            .ast_ptr(db)
            .to_node(&root)
            .output()
            .and_then(|o| o.ty());
        output
            .map(|o| lower_type_expr_with_self(db, file, o, Some(self.owner(db))))
            .unwrap_or_else(|| Type::Unit)
    }
}

#[salsa::tracked(debug)]
pub struct Function<'db> {
    pub name: Ustr,
    pub ast_ptr: ast::AstPtr<ast::FnItem>,
    pub file: ide::File,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub struct Param<'db> {
    pub name: Option<Ustr>,
    pub ty: Type<'db>,
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn params_by_name(self, db: &'db dyn salsa::Database) -> UstrIndexMap<Param<'db>> {
        self.params(db)
            .iter()
            .filter_map(|p| p.name.map(|n| (UstrHash(n), p.clone())))
            .collect()
    }

    #[salsa::tracked(returns(ref))]
    pub fn params(self, db: &'db dyn salsa::Database) -> Vec<Param<'db>> {
        let mut params = vec![];
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        for param in self
            .ast_ptr(db)
            .to_node(&root)
            .params()
            .into_iter()
            .flat_map(|p| p.params())
        {
            //TODO: error is there's self token
            let name = param.pattern().and_then(|p| {
                Some(match p {
                    ast::Pattern::NamePattern(name_patern) => name_patern,
                })
                .and_then(|n| n.name())
                .and_then(|n| n.text())
            });
            let ty = param
                .ty()
                .map(|ty| lower_type_expr(db, file, ty))
                .unwrap_or_else(|| Type::Unknown(Ustr::from("")));
            params.push(Param { name, ty });
        }
        params
    }

    #[salsa::tracked(returns(ref))]
    pub fn bare_fn_ty(self, db: &'db dyn salsa::Database) -> BareFn<'db> {
        BareFn {
            params: self.params(db).clone(),
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
            let Some((ast_ty, ir_ty)) = field
                .ty()
                .map(|ty| (ty.clone(), lower_type_expr(db, file, ty)))
            else {
                continue;
            };

            if let Type::Unknown(name) = &ir_ty {
                Diagnostic::new(
                    ast_ty.syntax().text_range(),
                    DiagnosticKind::TypeError,
                    format!("cannot find value `{}` in this scope", &name),
                )
                .accumulate(db);
            }

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
    Unknown(Ustr),
    Any,
    Unit,
    Never,
    Lit(LitKind),
    Struct(Struct<'db>),
    Dyn(Struct<'db>),
    Function(Function<'db>),
    Nilable(Box<Type<'db>>),
    BareFn(BareFn<'db>),
}

impl<'db> Type<'db> {
    pub fn is_unknown(&self) -> bool {
        match self {
            Self::Unknown(_) => true,
            Self::Nilable(nilable) if matches!(**nilable, Self::Unknown(_)) => true,
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

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub enum Expr {
    Missing,
    Unit,
    Path(Vec<Ustr>),
    Lit(LitKind),
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
    SelfVar,
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
        pattern: PatternId,
        ty: Option<Type<'db>>,
        expr: ExprId,
    },
    Expr {
        expr: ExprId,
        semi: Option<()>,
    },
}
