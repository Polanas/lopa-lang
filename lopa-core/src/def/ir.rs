use std::ops::Deref;

use itertools::Itertools;
use la_arena::{Idx, RawIdx};
use rowan::ast::AstNode;
use salsa::Accumulator;
use ustr::Ustr;

use crate::{
    common::LitKind,
    def::resolver,
    ide::{
        self,
        diagnostics::{Diagnostic, DiagnosticKind},
    },
    parsing::ast::{self, BinaryOpKind, UnaryOpKind},
    ty::infer,
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

#[salsa::tracked(debug)]
pub struct ImplBlock<'db> {
    pub file: ide::File,
    pub ast_ptr: ast::AstPtr<ast::ImplItem>,
}

#[salsa::tracked]
pub fn impl_block_diagnostics_acc<'db>(db: &'db dyn salsa::Database, item: ImplBlock<'db>) {
    let functions = item.functions(db);
    for func in functions {
        infer::infer_function(db, *func);
    }
}

#[salsa::tracked]
impl<'db> ImplBlock<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn functions(self, db: &'db dyn salsa::Database) -> Vec<Function<'db>> {
        let mut functions = vec![];
        let file = self.file(db);
        let parse = ide::parse(db, file);
        let impl_block = self.ast_ptr(db).to_node(&parse.syntax_node(db));

        for function in impl_block.functions() {
            let Some(name) = function.name().and_then(|n| n.text()) else {
                continue;
            };

            functions.push(Function::new(
                db,
                name,
                ast::AstPtr::new(&function),
                file,
                None,
                Some(self),
            ));
        }

        functions
    }

    #[salsa::tracked(returns(ref))]
    pub fn generics(self, db: &'db dyn salsa::Database) -> Generics<'db> {
        let file = self.file(db);
        let parse = ide::parse(db, file);
        let impl_block = self.ast_ptr(db).to_node(&parse.syntax_node(db));
        generic_types(db, file, impl_block.generics())
    }

    #[salsa::tracked(returns(ref))]
    pub fn implementee(self, db: &'db dyn salsa::Database) -> Option<Type<'db>> {
        let parse = ide::parse(db, self.file(db));
        let impl_block = self.ast_ptr(db).to_node(&parse.syntax_node(db));
        let implementee = impl_block.impl_ty().and_then(|t| t.ty())?;
        Some(resolver::resolve_type_expr(
            db,
            self.file(db),
            implementee,
            Some(self.generics(db)),
            None,
        ))
    }

    #[salsa::tracked(returns(ref))]
    pub fn owner(self, db: &'db dyn salsa::Database) -> Type<'db> {
        let file = self.file(db);
        let parse = ide::parse(db, file);
        let impl_block = self.ast_ptr(db).to_node(&parse.syntax_node(db));

        let Some(owner) = impl_block
            .impl_ty()
            .and_then(|t| t.ty())
            .or_else(|| impl_block.ty())
        else {
            return Type::Unknown;
        };

        resolver::resolve_type_expr(db, file, owner, Some(self.generics(db)), None)
    }
}

pub fn generic_types<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    generics: Option<ast::Generics>,
) -> Generics<'db> {
    let mut params = vec![];
    for param in generics.map(|g| g.params()).into_iter().flatten() {
        let Some((name, range)) = param
            .name()
            .and_then(|n| n.text().map(|t| (t, n.syntax().text_range())))
        else {
            continue;
        };
        let bounds = param
            .bounds()
            .map(|ty| resolver::resolve_type_expr(db, file, ty, None, None))
            .collect_vec();
        params.push(TypeParam {
            name,
            bounds: if bounds.len() == 0 {
                TypeBounds::default()
            } else {
                TypeBounds(Some(bounds))
            },
            text_range: range,
        });
    }
    Generics::new(params)
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum FunctionOwnerItem<'db> {
    Struct(Struct<'db>),
    Enum(Enum<'db>),
}

#[salsa::tracked(debug)]
pub struct Function<'db> {
    pub name: Ustr,
    pub ast_ptr: ast::AstPtr<ast::FnItem>,
    pub file: ide::File,
    pub owner_item: Option<FunctionOwnerItem<'db>>,
    pub impl_block: Option<ImplBlock<'db>>,
}

//TODO: refactor into tracked struct
#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub struct Generics<'db> {
    pub params: Vec<TypeParam<'db>>,
}

impl<'db> Generics<'db> {
    pub fn new(params: Vec<TypeParam<'db>>) -> Self {
        Self { params }
    }

    pub fn param(&self, name: &str) -> Option<&TypeParam<'db>> {
        self.params.iter().find(|p| p.name == name)
    }

    pub fn param_mut(&mut self, name: &str) -> Option<&mut TypeParam<'db>> {
        self.params.iter_mut().find(|p| p.name == name)
    }
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub struct TypeParam<'db> {
    pub name: Ustr,
    pub bounds: TypeBounds<'db>,
    pub text_range: rowan::TextRange,
}

#[derive(salsa::Update, Default, Hash, PartialEq, Eq, Clone, Debug)]
pub struct TypeBounds<'db>(Option<Vec<Type<'db>>>);

impl<'db> TypeBounds<'db> {
    pub fn new(bounds: Vec<Type<'db>>) -> Self {
        Self(Some(bounds))
    }
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Type<'db>> {
        self.0.iter().flatten()
    }
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub struct Param<'db> {
    pub name: Option<Ustr>,
    pub ty: Type<'db>,
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    fn is_method(self, db: &'db dyn salsa::Database) -> bool {
        self.params(db)
            .iter()
            .next()
            .and_then(|p| self.owner(db).as_ref().map(|o| p.ty == *o))
            .unwrap_or_default()
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
            if param.self_token().is_some() {
                let Some(owner) = self.owner(db) else {
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
                    ty: owner.clone(),
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
                    .type_expr()
                    .map(|ty| {
                        resolver::resolve_type_expr(
                            db,
                            file,
                            ty,
                            Some(self.generics(db)),
                            self.owner(db).as_ref(),
                        )
                    })
                    .unwrap_or_else(|| Type::Unknown);
                params.push(Param { name, ty });
            }
        }
        params
    }

    #[salsa::tracked(returns(ref))]
    pub fn params_by_name(self, db: &'db dyn salsa::Database) -> UstrIndexMap<Param<'db>> {
        self.params(db)
            .iter()
            .filter_map(|p| p.name.map(|n| (UstrHash(n), p.clone())))
            .collect()
    }

    #[salsa::tracked(returns(ref))]
    pub fn bare_fn_ty(self, db: &'db dyn salsa::Database) -> BareFn<'db> {
        BareFn {
            params: self.params(db).clone(),
            output: self.output(db).clone().into(),
        }
    }

    #[salsa::tracked(returns(ref))]
    pub fn generic_type(self, db: &'db dyn salsa::Database) -> Type<'db> {
        Type::Function(self, GenericParams::from_generics(db, self.generics(db)))
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
            .map(|o| {
                resolver::resolve_type_expr(
                    db,
                    file,
                    o,
                    Some(self.generics(db)),
                    self.owner(db).as_ref(),
                )
            })
            .unwrap_or_else(|| Type::Unit)
    }

    #[salsa::tracked(returns(ref))]
    pub fn owner(self, db: &'db dyn salsa::Database) -> Option<Type<'db>> {
        Some(if let Some(owner_item) = self.owner_item(db) {
            match owner_item {
                FunctionOwnerItem::Struct(struct_item) => struct_item.generic_type(db),
                FunctionOwnerItem::Enum(enum_item) => enum_item.generic_type(db),
            }
            .clone()
        } else {
            let impl_block = self.impl_block(db)?;
            impl_block.owner(db).clone()
        })
    }

    #[salsa::tracked(returns(ref))]
    pub fn generics(self, db: &'db dyn salsa::Database) -> Generics<'db> {
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        let owner_generics = self.impl_block(db).map(|b| b.generics(db)).or_else(|| {
            let owner = self.owner_item(db)?;
            Some(match owner {
                FunctionOwnerItem::Struct(struct_item) => struct_item.generics(db),
                FunctionOwnerItem::Enum(enum_item) => enum_item.generics(db),
            })
        });

        let generics = self.ast_ptr(db).to_node(&root).generics();
        let mut generics = generic_types(db, file, generics);

        if let Some(impl_generics) = owner_generics {
            for impl_param in &impl_generics.params {
                if let Some(duplicate) = generics.param(&impl_param.name) {
                    Diagnostic::new(
                        duplicate.text_range,
                        DiagnosticKind::TypeError,
                        format!(
                            "the name `{}` is already used for generic parameter",
                            duplicate.name
                        ),
                    )
                    .accumulate(db);
                }
                generics.params.push(impl_param.clone());
            }
        }

        generics
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

#[salsa::tracked]
pub fn enum_diagnostics_acc<'db>(db: &'db dyn salsa::Database, item: Enum<'db>) {
    let _fields = item.fields(db);
    for func in item.functions(db) {
        infer::infer_function(db, *func);
    }
}

#[salsa::tracked]
impl<'db> Enum<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn generics(self, db: &'db dyn salsa::Database) -> Generics<'db> {
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        let generics = self.ast_ptr(db).to_node(&root).generics();
        generic_types(db, file, generics)
    }

    #[salsa::tracked(returns(ref))]
    pub fn generic_type(self, db: &'db dyn salsa::Database) -> Type<'db> {
        Type::Enum(self, GenericParams::from_generics(db, self.generics(db)))
    }
    #[salsa::tracked(returns(ref))]
    pub fn fields(self, db: &'db dyn salsa::Database) -> Vec<Field<'db>> {
        let mut fields = vec![];
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        for element in self.ast_ptr(db).to_node(&root).elements() {
            if let ast::EnumElem::Field(field) = element {
                let Some(name) = field.name().and_then(|n| n.text()) else {
                    continue;
                };

                let ptr = ast::AstPtr::new(&field);
                let Some((_, ir_ty)) = field.ty().map(|ty| {
                    (
                        ty.clone(),
                        resolver::resolve_item_type_expr(
                            db,
                            file,
                            ty,
                            Some(self.generics(db)),
                            Some(self.generic_type(db)),
                        ),
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
    pub fn functions(self, db: &'db dyn salsa::Database) -> Vec<Function<'db>> {
        let mut functions = vec![];
        let file = self.file(db);
        let parse = ide::parse(db, file);
        let struct_item = self.ast_ptr(db).to_node(&parse.syntax_node(db));

        for elem in struct_item.elements() {
            let ast::EnumElem::FnItem(fn_item) = elem else {
                continue;
            };
            let Some(name) = fn_item.name().and_then(|n| n.text()) else {
                continue;
            };

            functions.push(Function::new(
                db,
                name,
                ast::AstPtr::new(&fn_item),
                file,
                Some(FunctionOwnerItem::Enum(self)),
                None,
            ));
        }

        functions
    }
}

#[salsa::tracked(debug)]
pub struct Struct<'db> {
    pub name: Ustr,
    pub ast_ptr: ast::AstPtr<ast::StructItem>,
    pub file: ide::File,
}

#[salsa::tracked]
pub fn struct_diagnostics_acc<'db>(db: &'db dyn salsa::Database, item: Struct<'db>) {
    let _fields = item.fields(db);
    for func in item.functions(db) {
        infer::infer_function(db, *func);
    }
}

#[salsa::tracked]
impl<'db> Struct<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn generics(self, db: &'db dyn salsa::Database) -> Generics<'db> {
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        let generics = self.ast_ptr(db).to_node(&root).generics();
        generic_types(db, file, generics)
    }

    #[salsa::tracked(returns(ref))]
    pub fn generic_type(self, db: &'db dyn salsa::Database) -> Type<'db> {
        Type::Struct(self, GenericParams::from_generics(db, self.generics(db)))
    }

    #[salsa::tracked(returns(ref))]
    pub fn functions(self, db: &'db dyn salsa::Database) -> Vec<Function<'db>> {
        let mut functions = vec![];
        let file = self.file(db);
        let parse = ide::parse(db, file);
        let struct_item = self.ast_ptr(db).to_node(&parse.syntax_node(db));

        for elem in struct_item.elements() {
            let ast::StructElem::FnItem(fn_item) = elem else {
                continue;
            };
            let Some(name) = fn_item.name().and_then(|n| n.text()) else {
                continue;
            };

            functions.push(Function::new(
                db,
                name,
                ast::AstPtr::new(&fn_item),
                file,
                Some(FunctionOwnerItem::Struct(self)),
                None,
            ));
        }

        functions
    }

    #[salsa::tracked(returns(ref))]
    pub fn fields(self, db: &'db dyn salsa::Database) -> Vec<Field<'db>> {
        let mut fields = vec![];
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        for element in self.ast_ptr(db).to_node(&root).elements() {
            if let ast::StructElem::Field(field) = element {
                let Some(name) = field.name().and_then(|n| n.text()) else {
                    continue;
                };

                let ptr = ast::AstPtr::new(&field);
                let Some((_, ir_ty)) = field.ty().map(|ty| {
                    (
                        ty.clone(),
                        resolver::resolve_item_type_expr(
                            db,
                            file,
                            ty,
                            Some(self.generics(db)),
                            Some(self.generic_type(db)),
                        ),
                    )
                }) else {
                    continue;
                };

                fields.push(Field::new(db, name, ir_ty, ptr));
            }
        }

        fields
    }
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

// #[salsa::tracked(returns(ref))]
// pub fn struct_impl_item<'db>(
//     db: &'db dyn salsa::Database,
//     struct_item: Struct<'db>,
//     implementee: ide::Implementee<'db>,
//     name: Ustr,
// ) -> Option<ImplItem<'db>> {
//     let impl_map = ide::impl_map(db, struct_item.file(db).source_root(db));
//     let key = ide::ImplKey {
//         implementor: Type::Struct(struct_item, GenericParams::default()),
//         implementee,
//     };
//     impl_map.get(&key)?.get(&name).cloned()
// }

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub struct BareFn<'db> {
    pub params: Vec<Param<'db>>,
    pub output: Box<Type<'db>>,
}

#[salsa::tracked(debug)]
pub struct GenericParams<'db> {
    pub params: Option<Vec<Type<'db>>>,
}

impl<'db> GenericParams<'db> {
    pub fn from_generics(db: &'db dyn salsa::Database, generics: &Generics<'db>) -> Self {
        if generics.params.is_empty() {
            return Self::new(db, None);
        };

        let params = generics
            .params
            .iter()
            .map(|p| Type::Generic(p.name))
            .collect_vec();
        Self::new(db, Some(params))
    }

    pub fn default(db: &'db dyn salsa::Database) -> Self {
        Self::new(db, None)
    }

    pub fn is_some(&self, db: &'db dyn salsa::Database) -> bool {
        self.params(db).is_some()
    }
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum Type<'db> {
    Unknown,
    Any,
    Unit,
    Never,
    Generic(Ustr),
    Lit(LitKind),
    Struct(Struct<'db>, GenericParams<'db>),
    Enum(Enum<'db>, GenericParams<'db>),
    Dyn(Vec<(Struct<'db>, GenericParams<'db>)>),
    Function(Function<'db>, GenericParams<'db>),
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

#[salsa::tracked(debug)]
pub struct GenericPath<'db> {
    pub segments: Vec<GenericPathSegment<'db>>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub struct GenericPathSegment<'db> {
    pub ident: Ustr,
    pub args: GenericParams<'db>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash, Default)]
pub struct Path(pub Vec<Ustr>);

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub enum Expr {
    Missing,
    Unit,
    Path(GenericPath<'static>),
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
        path: Path,
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
