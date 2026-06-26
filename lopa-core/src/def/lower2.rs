use std::collections::HashMap;

use itertools::Itertools as _;
use la_arena::{Arena, Idx};
use ustr::Ustr;

use crate::{
    common::LitKind,
    ide,
    parsing::ast::{self, BinaryOpKind, UnaryOpKind},
};

#[salsa::tracked(debug)]
pub struct IrFile<'db> {
    pub module: Module<'db>,
}

pub type ExprId = Idx<Expr>;

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum Expr {
    Missing,
    Unit,
    Lit(LitKind),
    Path(Path),
    As {
        expr: ExprId,
        ty: TypeExprId,
    },
    Is {
        expr: ExprId,
        pat: PatId,
    },
    IsNot {
        expr: ExprId,
        pat: PatId,
    },
    SelfExpr,
    Closure {
        params: Vec<ClosureParam>,
        body: ExprId,
        output: Option<TypeExprId>,
    },
    Field {
        name: Ustr,
        expr: ExprId,
    },
    Method {
        expr: ExprId,
        name: Ustr,
        args: Vec<Arg>,
    },
    Record {
        path: Path,
        fields: Vec<RecordField>,
    },
    Binary {
        lhs: ExprId,
        rhs: ExprId,
        kind: BinaryOpKind,
    },
    Unary {
        kind: UnaryOpKind,
        expr: ExprId,
    },
    Block {
        stmts: Vec<StmtId>,
    },
    Index {
        base: ExprId,
        index: ExprId,
    },
    Call {
        func: ExprId,
        agrs: Vec<Arg>,
    },
    Paren(ExprId),
    Return {
        expr: ExprId,
    },
    If {
        cond: ExprId,
        if_branch: ExprId,
        else_branch: ExprId,
    },
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub struct ClosureParam {
    pattern: PatId,
    ty: Option<TypeExprId>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub struct RecordField {
    pub name: Ustr,
    pub expr: ExprId,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
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

pub type PatId = Idx<Pat>;

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum Pat {
    Missing,
    Path(Path),
    Name(Ustr),
    Wildcard,
}

pub type StmtId = Idx<Stmt>;

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum Stmt {
    Let {
        pat: PatId,
        ty: Option<TypeExprId>,
        expr: ExprId,
    },
    Expr {
        expr: ExprId,
        semi: Option<()>,
    },
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum Item<'db> {
    Function(Function<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    UseItem(UseItem<'db>),
    Module(Module<'db>),
    Impl(ImplBlock<'db>),
}

#[salsa::tracked(debug)]
pub struct ImplBlock<'db> {
    implementee: Option<TypeExprId>,
    implementor: TypeExprId,
    pub items: Vec<Item<'db>>,
}

#[salsa::tracked(debug)]
pub struct Function<'db> {
    pub name: Ustr,
    pub params: Vec<FnParam>,
    pub output: Option<TypeExprId>,
    pub body: Body<'db>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum FnParam {
    SelfParam,
    PatParam {
        pat: PatId,
        type_expr: TypeExprId,
        default_value: Option<ExprId>,
    },
}

#[salsa::tracked(debug)]
pub struct Body<'db> {
    #[returns(ref)]
    pub exprs: Arena<Expr>,
    #[returns(ref)]
    pub pats: Arena<Pat>,
    #[returns(ref)]
    pub type_exprs: Arena<TypeExpr>,
    #[returns(ref)]
    pub stmts: Arena<Stmt>,
    pub body_expr: ExprId,
}

struct LowerBodyCtx<'db> {
    db: &'db dyn salsa::Database,
    expr: ast::Expr,
    ctx: BodyCtx,
}

impl<'db> LowerBodyCtx<'db> {
    fn new(db: &'db dyn salsa::Database, expr: ast::Expr) -> Self {
        Self {
            db,
            expr,
            ctx: BodyCtx::default(),
        }
    }

    fn alloc_expr(&mut self, expr: Expr) -> ExprId {
        self.ctx.exprs.alloc(expr)
    }

    fn alloc_pat(&mut self, pat: Pat) -> PatId {
        self.ctx.pats.alloc(pat)
    }

    fn alloc_type_expr(&mut self, type_expr: TypeExpr) -> TypeExprId {
        self.ctx.type_exprs.alloc(type_expr)
    }

    fn missing_type_expr(&mut self) -> TypeExprId {
        self.ctx.type_exprs.alloc(TypeExpr::Missing)
    }

    fn missing_expr(&mut self) -> ExprId {
        self.alloc_expr(Expr::Missing)
    }

    fn missing_pat(&mut self) -> PatId {
        self.alloc_pat(Pat::Missing)
    }

    fn alloc_stmt(&mut self, stmt: Stmt) -> StmtId {
        self.ctx.stmts.alloc(stmt)
    }

    fn lower(mut self) -> Body<'db> {
        let expr = self.expr(self.expr.clone());
        Body::new(
            self.db,
            self.ctx.exprs,
            self.ctx.pats,
            self.ctx.type_exprs,
            self.ctx.stmts,
            expr,
        )
    }

    fn stmt(&mut self, stmt: ast::Stmt) -> Option<StmtId> {
        Some(match stmt {
            ast::Stmt::LetStmt(let_stmt) => {
                let (pat, expr) = (let_stmt.pattern()?, let_stmt.expr()?);
                let pat = self.pat(pat);
                let expr = self.expr(expr);
                let ty = let_stmt.ty().map(|ty| self.type_expr(ty));
                self.alloc_stmt(Stmt::Let { pat, ty, expr })
            }
            ast::Stmt::ExprStmt(expr_stmt) => {
                let expr = self.expr(expr_stmt.expr()?);
                self.alloc_stmt(Stmt::Expr {
                    expr,
                    semi: expr_stmt.semi_token().map(|_| ()),
                })
            }
        })
    }

    fn pat(&mut self, pat: ast::Pattern) -> PatId {
        match pat {
            ast::Pattern::NamePattern(name_pattern) => {
                let pat = name_pattern
                    .name()
                    .and_then(|n| n.text())
                    .map(Pat::Name)
                    .unwrap_or_else(|| Pat::Missing);
                self.alloc_pat(pat)
            }
            ast::Pattern::PathPattern(path_pattern) => {
                let path = path_pattern.path().map(|p| self.path(p));
                self.alloc_pat(path.map(Pat::Path).unwrap_or_else(|| Pat::Missing))
            }
            ast::Pattern::WildcardPattern(_) => self.alloc_pat(Pat::Wildcard),
        }
    }

    fn path(&mut self, path: ast::Path) -> Path {
        Path {
            segments: path
                .segments()
                .filter_map(|s| self.path_segment(s))
                .collect_vec(),
        }
    }

    fn path_segment(&mut self, path_segment: ast::PathSegment) -> Option<PathSegment> {
        Some(PathSegment {
            ident: path_segment.ident()?,
            generic_args: path_segment
                .generic_args()
                .map(|args| args.types())
                .into_iter()
                .flatten()
                .map(|ty| self.type_expr(ty))
                .collect_vec(),
        })
    }

    fn pat_opt(&mut self, pat: Option<ast::Pattern>) -> PatId {
        pat.map(|e| self.pat(e))
            .unwrap_or_else(|| self.missing_pat())
    }

    fn type_expr(&mut self, type_expr: ast::TypeExpr) -> TypeExprId {
        fn inner(this: &mut LowerBodyCtx, type_expr: ast::TypeExpr) -> Option<TypeExprId> {
            Some(match type_expr {
                ast::TypeExpr::DynType(dyn_type) => {
                    let path = this.path(dyn_type.path()?);
                    this.alloc_type_expr(TypeExpr::Dyn(path))
                }
                ast::TypeExpr::ParenType(paren_type) => {
                    let ty = paren_type
                        .type_expr()
                        .map(|ty| this.type_expr(ty))
                        .unwrap_or_else(|| this.missing_type_expr());
                    this.alloc_type_expr(TypeExpr::Paren(ty))
                }
                ast::TypeExpr::PathType(path_type) => {
                    let path = this.path(path_type.value()?);
                    this.alloc_type_expr(TypeExpr::Path(path))
                }
                ast::TypeExpr::NilableType(nilable_type) => {
                    let ty = nilable_type
                        .ty()
                        .map(|nilable| this.type_expr(nilable))
                        .unwrap_or_else(|| this.missing_type_expr());
                    this.alloc_type_expr(TypeExpr::Nilable(ty))
                }
                ast::TypeExpr::LitType(lit_type) => {
                    this.alloc_type_expr(TypeExpr::Lit(lit_type.kind()?))
                }
                ast::TypeExpr::FnType(fn_type) => {
                    let output = fn_type.output().map(|o| {
                        o.ty()
                            .map(|ty| this.type_expr(ty))
                            .unwrap_or_else(|| this.missing_type_expr())
                    });
                    let params = fn_type
                        .param_list()
                        .map(|p| p.params())
                        .into_iter()
                        .flatten()
                        .filter_map(|p| this.fn_type_param(p))
                        .collect_vec();
                    this.alloc_type_expr(TypeExpr::Fn { params, output })
                }
                ast::TypeExpr::AnyType(_) => this.alloc_type_expr(TypeExpr::Any),
                ast::TypeExpr::UnitType(_) => this.alloc_type_expr(TypeExpr::Unit),
                ast::TypeExpr::SelfType(_) => this.alloc_type_expr(TypeExpr::SelfTy),
            })
        }
        inner(self, type_expr).unwrap_or_else(|| self.missing_type_expr())
    }

    fn type_expr_opt(&mut self, type_expr: Option<ast::TypeExpr>) -> TypeExprId {
        type_expr
            .map(|ty| self.type_expr(ty))
            .unwrap_or_else(|| self.missing_type_expr())
    }

    fn fn_type_param(&mut self, param: ast::FnTypeParam) -> Option<FnTypeParam> {
        let name = param.name().and_then(|n| n.text())?;
        Some(FnTypeParam {
            name,
            ty: self.type_expr_opt(param.ty()),
        })
    }

    fn expr(&mut self, expr: ast::Expr) -> ExprId {
        match expr {
            ast::Expr::AsExpr(as_expr) => self.missing_expr(),
            ast::Expr::IsExpr(is_expr) => self.missing_expr(),
            ast::Expr::IsNotExpr(is_not_expr) => self.missing_expr(),
            ast::Expr::SelfExpr(self_expr) => self.missing_expr(),
            ast::Expr::ClosureExpr(closure_expr) => self.missing_expr(),
            ast::Expr::FieldExpr(field_expr) => self.missing_expr(),
            ast::Expr::MethodExpr(method_expr) => self.missing_expr(),
            ast::Expr::RecordExpr(record_expr) => self.missing_expr(),
            ast::Expr::UnitExpr(unit_expr) => self.missing_expr(),
            ast::Expr::PathExpr(path_expr) => self.missing_expr(),
            ast::Expr::BinaryExpr(binary_expr) => self.missing_expr(),
            ast::Expr::UnaryExpr(unary_expr) => self.missing_expr(),
            ast::Expr::BlockExpr(block_expr) => {
                let stmts = block_expr
                    .stmts()
                    .filter_map(|stmt| self.stmt(stmt))
                    .collect_vec();
                self.alloc_expr(Expr::Block { stmts })
            }
            ast::Expr::IndexExpr(index_expr) => self.missing_expr(),
            ast::Expr::CallExpr(call_expr) => self.missing_expr(),
            ast::Expr::ParenExpr(paren_expr) => {
                let expr = self.expr_opt(paren_expr.expr());
                self.alloc_expr(Expr::Paren(expr))
            }
            ast::Expr::ReturnExpr(return_expr) => {
                let expr = self.expr_opt(return_expr.expr());
                self.alloc_expr(Expr::Return { expr })
            }
            ast::Expr::LitExpr(lit_expr) => self.alloc_expr(
                lit_expr
                    .kind()
                    .map(Expr::Lit)
                    .unwrap_or_else(|| Expr::Missing),
            ),
            ast::Expr::IfExpr(if_expr) => self.missing_expr(),
        }
    }

    fn expr_opt(&mut self, expr: Option<ast::Expr>) -> ExprId {
        expr.map(|e| self.expr(e))
            .unwrap_or_else(|| self.missing_expr())
    }
}

#[salsa::tracked(debug)]
pub struct Struct<'db> {
    pub name: Ustr,
}

#[salsa::tracked(debug)]
pub struct Enum<'db> {
    pub name: Ustr,
}

#[salsa::tracked(debug)]
pub struct UseItem<'db> {}

#[salsa::tracked(debug)]
pub struct Module<'db> {
    pub name: Ustr,
    #[returns(ref)]
    pub items: Option<Vec<Item<'db>>>,
}

#[derive(Default)]
pub struct BodyCtx {
    exprs: Arena<Expr>,
    pats: Arena<Pat>,
    type_exprs: Arena<TypeExpr>,
    stmts: Arena<Stmt>,
}

pub struct ModuleCtx<'db> {
    name: Ustr,
    items: Vec<Item<'db>>,
}

pub type TypeExprId = Idx<TypeExpr>;

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum TypeExpr {
    Missing,
    Any,
    Unit,
    Never,
    SelfTy,
    Lit(LitKind),
    Path(Path),
    Dyn(Path),
    Nilable(TypeExprId),
    Paren(TypeExprId),
    Fn {
        params: Vec<FnTypeParam>,
        output: Option<TypeExprId>,
    },
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub struct FnTypeParam {
    pub name: Ustr,
    pub ty: TypeExprId,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub struct Path {
    segments: Vec<PathSegment>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub struct PathSegment {
    ident: Ustr,
    generic_args: Vec<TypeExprId>,
}

struct LowerCtx<'db> {
    db: &'db dyn salsa::Database,
    ast_file: ast::File,
    file: ide::File,
}

impl<'db> LowerCtx<'db> {
    fn new(db: &'db dyn salsa::Database, file: ide::File) -> Self {
        Self {
            db,
            ast_file: ide::parse(db, file).file(db),
            file,
        }
    }

    fn lower(mut self) -> Module<'db> {
        let items = self.items(self.ast_file.items());
        Module::new(self.db, Ustr::from("TODO: get mod name"), Some(items))
    }

    fn items(&mut self, items_iter: impl Iterator<Item = ast::Item>) -> Vec<Item<'db>> {
        let mut items = vec![];
        for item in items_iter {
            match item {
                ast::Item::FnItem(fn_item) => {
                    if let Some(fn_item) = self.fn_item(fn_item) {
                        items.push(Item::Function(fn_item));
                    }
                }
                ast::Item::ModItem(mod_item) => {
                    if let Some(mod_item) = self.mod_item(mod_item) {
                        items.push(Item::Module(mod_item));
                    }
                }
                ast::Item::ImplItem(impl_item) => {}
                ast::Item::StructItem(struct_item) => {}
                ast::Item::EnumItem(enum_item) => {}
                ast::Item::UseItem(use_item) => {}
            };
        }
        items
    }

    fn fn_item(&mut self, fn_item: ast::FnItem) -> Option<Function<'db>> {
        let name = fn_item.name().and_then(|n| n.text())?;
        let mut body_ctx = LowerBodyCtx::new(self.db, ast::Expr::BlockExpr(fn_item.body()?));
        let mut params = vec![];
        for param in fn_item.params().map(|p| p.params()).into_iter().flatten() {
            if param.self_token().is_some() {
                params.push(FnParam::SelfParam);
                continue;
            }
            let Some(pat) = param.pattern() else {
                continue;
            };
            let pat = body_ctx.pat(pat);
            let type_expr = body_ctx.type_expr_opt(param.type_expr());
            let default_value = param.default_value().map(|expr| body_ctx.expr(expr));

            params.push(FnParam::PatParam {
                pat,
                type_expr,
                default_value,
            });
        }

        let output = fn_item.output().map(|o| body_ctx.type_expr_opt(o.ty()));
        let body = body_ctx.lower();

        Some(Function::new(self.db, name, params, output, body))
    }

    fn mod_item(&mut self, mod_item: ast::ModItem) -> Option<Module<'db>> {
        let name = mod_item.name().and_then(|n| n.text())?;
        Some(Module::new(
            self.db,
            name,
            match mod_item.semi() {
                Some(_) => None,
                None => Some(self.items(mod_item.items())),
            },
        ))
    }
}

#[salsa::tracked(returns(ref))]
pub fn lower<'db>(db: &'db dyn salsa::Database, file: ide::File) -> IrFile<'db> {
    let ctx = LowerCtx::new(db, file);
    IrFile::new(db, ctx.lower())
}
