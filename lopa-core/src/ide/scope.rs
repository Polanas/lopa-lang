use std::{collections::HashMap, sync::Arc};

use la_arena::{Arena, Idx};

use crate::{
    def::{
        ExprId, PatId, Symbol,
        hir::{self, Class, Enum, Expr, Function, Item, Module, Pat, Struct, Trait},
    },
    ide::{DiagnosticLocation, ResolveItem},
};

#[derive(salsa::Supertype, Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub enum ModuleDef<'db> {
    Struct(Struct<'db>),
    Class(Class<'db>),
    Trait(Trait<'db>),
    Enum(Enum<'db>),
    Module(Module<'db>),
    Function(Function<'db>),
}

impl<'db> ModuleDef<'db> {
    pub fn as_type(&self) -> Option<TypeModuleDef<'db>> {
        Some(match self {
            ModuleDef::Struct(item) => TypeModuleDef::Struct(*item),
            ModuleDef::Class(item) => TypeModuleDef::Class(*item),
            ModuleDef::Trait(item) => TypeModuleDef::Trait(*item),
            ModuleDef::Enum(item) => TypeModuleDef::Enum(*item),
            ModuleDef::Module(item) => TypeModuleDef::Module(*item),
            ModuleDef::Function(_) => return None,
        })
    }

    pub fn as_value(&self) -> Option<ValueModuleDef<'db>> {
        Some(match self {
            ModuleDef::Function(item) => ValueModuleDef::Function(*item),
            ModuleDef::Struct(_)
            | ModuleDef::Class(_)
            | ModuleDef::Trait(_)
            | ModuleDef::Enum(_)
            | ModuleDef::Module(_) => return None,
        })
    }
}

#[derive(salsa::Supertype, Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub enum TypeModuleDef<'db> {
    Struct(Struct<'db>),
    Class(Class<'db>),
    Trait(Trait<'db>),
    Enum(Enum<'db>),
    Module(Module<'db>),
}

impl<'db> TypeModuleDef<'db> {
    pub fn as_mod_def(&self) -> ModuleDef<'db> {
        match self {
            TypeModuleDef::Struct(item) => ModuleDef::Struct(*item),
            TypeModuleDef::Class(item) => ModuleDef::Class(*item),
            TypeModuleDef::Trait(item) => ModuleDef::Trait(*item),
            TypeModuleDef::Enum(item) => ModuleDef::Enum(*item),
            TypeModuleDef::Module(item) => ModuleDef::Module(*item),
        }
    }

    pub fn name(&self, db: &'db dyn salsa::Database) -> Symbol {
        match self {
            TypeModuleDef::Struct(item) => item.name(db),
            TypeModuleDef::Class(item) => item.name(db),
            TypeModuleDef::Trait(item) => item.name(db),
            TypeModuleDef::Enum(item) => item.name(db),
            TypeModuleDef::Module(item) => item.name(db),
        }
    }

    pub fn location(&self, db: &'db dyn salsa::Database) -> DiagnosticLocation {
        match self {
            TypeModuleDef::Struct(item) => DiagnosticLocation::Struct(item.id(db)),
            TypeModuleDef::Class(item) => DiagnosticLocation::Class(item.id(db)),
            TypeModuleDef::Trait(item) => DiagnosticLocation::Trait(item.id(db)),
            TypeModuleDef::Enum(item) => DiagnosticLocation::Enum(item.id(db)),
            TypeModuleDef::Module(item) => DiagnosticLocation::Module(item.id(db).expect("if Im asking for a location of a module, it shouldn't be a directory one without a file defined (which is the only way it could not have ast id)")),
        }
    }
}

#[derive(salsa::Supertype, Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub enum ValueModuleDef<'db> {
    Function(Function<'db>),
}

impl<'db> ValueModuleDef<'db> {
    pub fn as_mod_def(&self) -> ModuleDef<'db> {
        match self {
            ValueModuleDef::Function(item) => ModuleDef::Function(*item),
        }
    }

    pub fn name(&self, db: &'db dyn salsa::Database) -> Symbol {
        match self {
            ValueModuleDef::Function(item) => item.name(db),
        }
    }

    pub fn location(&self, db: &'db dyn salsa::Database) -> DiagnosticLocation {
        match self {
            ValueModuleDef::Function(item) => DiagnosticLocation::Function(item.id(db)),
        }
    }
}

#[derive(Clone, PartialEq, Default, salsa::SalsaValue)]
pub struct ModuleDefs<'db> {
    pub values: indexmap::IndexMap<Symbol, ValueModuleDef<'db>>,
    pub types: indexmap::IndexMap<Symbol, TypeModuleDef<'db>>,
}

impl<'db> ModuleDefs<'db> {
    pub fn resolve_item(&self, name: Symbol) -> Option<ResolveItem<'db>> {
        let value = self.values.get(&name).cloned();
        let ty = self.types.get(&name).cloned();

        Some(match (value, ty) {
            (Some(value), Some(ty)) => ResolveItem::Both { ty, value },
            (Some(value), None) => ResolveItem::Value(value),
            (None, Some(ty)) => ResolveItem::Type(ty),
            _ => return None,
        })
    }
}

#[salsa::tracked]
impl<'db> Module<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn defs(self, db: &'db dyn salsa::Database) -> ModuleDefs<'db> {
        let mut values: indexmap::IndexMap<_, _> = Default::default();
        let mut types: indexmap::IndexMap<_, _> = Default::default();

        for item in self.items(db).items(db).iter() {
            match item {
                Item::Function(item) => {
                    values.insert(item.name(db), ValueModuleDef::Function(*item));
                }
                Item::Struct(item) => {
                    types.insert(item.name(db), TypeModuleDef::Struct(*item));
                }
                Item::Enum(item) => {
                    types.insert(item.name(db), TypeModuleDef::Enum(*item));
                }
                Item::Module(item) => {
                    types.insert(item.name(db), TypeModuleDef::Module(*item));
                }
                Item::Class(item) => {
                    types.insert(item.name(db), TypeModuleDef::Class(*item));
                }
                Item::Trait(item) => {
                    types.insert(item.name(db), TypeModuleDef::Trait(*item));
                }
                Item::Use(_) | Item::Impl(_) => {}
            };
        }

        ModuleDefs { values, types }
    }
}

pub struct ExprScopesCtx<'db> {
    scopes: ExprScopes<'db>,
    db: &'db dyn salsa::Database,
}

impl<'db> ExprScopesCtx<'db> {
    fn new(db: &'db dyn salsa::Database) -> Self {
        Self {
            scopes: Default::default(),
            db,
        }
    }

    fn finish(self) -> ExprScopes<'db> {
        self.scopes
    }

    fn traverse_params(&mut self, params: hir::FnParamList<'db>) {
        let root = self.root_scope();
        for param in params.params(self.db) {
            match param.kind(self.db) {
                hir::FnParamKind::Pat { pat, .. } => {
                    if let Some(pat) = pat {
                        self.traverse_pat(pat, root);
                    }
                }
                hir::FnParamKind::SelfParam => {}
            }
        }
    }

    fn traverse(mut self, expr: hir::Expr<'db>) -> ExprScopes {
        if let hir::ExprKind::Block { stmts } = expr.kind(self.db) {
            let root = self.root_scope();
            self.traverse_block(root, stmts);
            self.finish()
        } else {
            panic!("expected block expression");
        }
    }

    fn traverse_expr(&mut self, expr: hir::Expr<'db>, scope: ScopeId<'db>) {
        self.scopes.scope_by_expr.insert(expr, scope);
        match expr.kind(self.db) {
            hir::ExprKind::Unit
            | hir::ExprKind::Lit(_)
            | hir::ExprKind::Path(_)
            | hir::ExprKind::SelfExpr => {}
            hir::ExprKind::As { expr, .. } => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::Is { expr, pat } => {
                self.traverse_expr(expr, scope);
                self.traverse_pat(pat, scope);
            }
            hir::ExprKind::IsNot { expr, pat } => {
                self.traverse_expr(expr, scope);
                self.traverse_pat(pat, scope);
            }
            hir::ExprKind::Closure { params, body, .. } => {
                let scope = self.scopes.scopes.alloc(ScopeData::from_parent(scope));
                for param in params.params(self.db).iter() {
                    self.traverse_pat(param.pat(self.db), scope);
                }
                self.traverse_expr(body, scope);
            }
            hir::ExprKind::Field { expr, .. } => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::Method { expr, args, .. } => {
                self.traverse_expr(expr, scope);
                for arg in args.args(self.db).iter() {
                    self.traverse_expr(arg.kind(self.db).value(), scope);
                }
            }
            hir::ExprKind::Record { fields, .. } => {
                for field in fields.fields(self.db).iter() {
                    self.traverse_expr(field.expr(self.db), scope);
                }
            }
            hir::ExprKind::Binary { lhs, rhs, .. } => {
                self.traverse_expr(lhs, scope);
                self.traverse_expr(rhs, scope);
            }
            hir::ExprKind::Unary { expr, .. } => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::Index { base, index } => {
                self.traverse_expr(base, scope);
                self.traverse_expr(index, scope);
            }
            hir::ExprKind::Call { func, args } => {
                self.traverse_expr(func, scope);
                for arg in args.args(self.db).iter() {
                    self.traverse_expr(arg.kind(self.db).value(), scope);
                }
            }
            hir::ExprKind::Block { stmts } => {
                let scope = self.scopes.scopes.alloc(ScopeData::from_parent(scope));
                self.traverse_block(scope, stmts);
            }
            hir::ExprKind::Paren(expr) => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::Return(expr) => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::If(if_expr) => {
                self.traverse_if_expr(if_expr, scope);
            }
            hir::ExprKind::Loop { block } => {
                self.traverse_expr(block, scope);
            }
            hir::ExprKind::While { cond, block } => {
                self.traverse_expr(cond, scope);
                self.traverse_expr(block, scope);
            }
            hir::ExprKind::For {
                loop_expr,
                iterable,
                block,
            } => {
                self.traverse_expr(loop_expr, scope);
                self.traverse_expr(iterable, scope);
                self.traverse_expr(block, scope);
            }
            hir::ExprKind::Tuple { exprs } => {
                for expr in exprs.exprs(self.db).iter() {
                    self.traverse_expr(*expr, scope);
                }
            }
        }
    }

    fn traverse_block(&mut self, scope: Idx<ScopeData<'db>>, stmts: hir::StmtList<'db>) {
        for stmt in stmts.stmts(self.db).iter() {
            self.traverse_stmt(*stmt, scope);
        }
    }

    fn traverse_if_expr(&mut self, expr: hir::IfExpr<'db>, scope: ScopeId<'db>) {
        self.traverse_expr(expr.cond(self.db), scope);
        self.traverse_expr(expr.if_branch(self.db), scope);
        if let Some(else_branch) = expr.else_branch(self.db) {
            match else_branch {
                hir::ElseBranch::Block(expr) => self.traverse_expr(expr, scope),
                hir::ElseBranch::If(if_expr) => self.traverse_if_expr(if_expr, scope),
            }
        }
    }

    fn traverse_stmt(&mut self, stmt: hir::Stmt<'db>, scope: ScopeId<'db>) {
        match stmt.kind(self.db) {
            hir::StmtKind::Let { pat, expr, .. } => {
                self.traverse_pat(pat, scope);
                self.traverse_expr(expr, scope);
            }
            hir::StmtKind::Expr { expr, .. } => {
                self.traverse_expr(expr, scope);
            }
        }
    }

    fn traverse_pat(&mut self, pat: hir::Pat<'db>, scope: ScopeId<'db>) {
        match pat.kind(self.db) {
            hir::PatKind::Name(symbol) => {
                self.scopes.scopes[scope]
                    .entries
                    .push(ScopeEntry { name: symbol, pat });
            }
            hir::PatKind::Wildcard => {}
            hir::PatKind::Path(_) => {}
            hir::PatKind::Paren(pat) => self.traverse_pat(pat, scope),
            hir::PatKind::Tuple(pat_list) => {
                for pat in pat_list.patterns(self.db).iter() {
                    self.traverse_pat(*pat, scope);
                }
            }
        }
    }

    fn root_scope(&mut self) -> ScopeId<'db> {
        if !self.scopes.scopes.is_empty() {
            return self.scopes.scopes.iter().next().unwrap().0;
        }
        self.scopes.scopes.alloc(ScopeData {
            parent: None,
            entries: vec![],
        })
    }
}

#[salsa::tracked]
impl<'db> hir::Function<'db> {
    #[salsa::tracked(returns(ref))]
    fn expr_scopes(self, db: &'db dyn salsa::Database) -> Arc<ExprScopes<'db>> {
        let mut ctx = ExprScopesCtx::new(db);
        ctx.traverse_params(self.contents(db).params);
        if let Some(body) = self.contents(db).body_expr {
            ctx.traverse(body)
        } else {
            ctx.finish()
        }
        .into()
    }
}

#[derive(Debug, PartialEq, Eq, Default, salsa::SalsaValue, Clone)]
pub struct ExprScopes<'db> {
    scopes: Arena<ScopeData<'db>>,
    scope_by_expr: HashMap<Expr<'db>, ScopeId<'db>>,
}

impl<'db> ExprScopes<'db> {
    pub fn entries(&self, scope: ScopeId<'db>) -> &[ScopeEntry] {
        &self.scopes[scope].entries
    }

    fn root_scope(&self) -> ScopeId<'db> {
        self.scopes.iter().next().unwrap().0
    }

    pub fn scope_for_expr(&self, expr: Expr<'db>) -> Option<ScopeId<'db>> {
        self.scope_by_expr.get(&expr).copied()
    }

    pub fn scope_chain(&self, scope: Option<ScopeId<'db>>) -> impl Iterator<Item = ScopeId<'db>> {
        std::iter::successors(scope, move |&scope| self.scopes[scope].parent)
    }

    pub fn resolve_name_in_scope(&self, scope: ScopeId<'db>, name: Symbol) -> Option<&ScopeEntry> {
        self.scope_chain(Some(scope))
            .find_map(|scope| self.entries(scope).iter().rev().find(|it| it.name == name))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, salsa::SalsaValue)]
pub struct ScopeEntry<'db> {
    name: Symbol,
    pat: Pat<'db>,
}

impl<'db> ScopeEntry<'db> {
    pub fn name(&self) -> Symbol {
        self.name
    }

    pub fn pat(&self) -> Pat<'db> {
        self.pat
    }
}

pub type ScopeId<'db> = Idx<ScopeData<'db>>;

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue, Clone)]
pub struct ScopeData<'db> {
    parent: Option<ScopeId<'db>>,
    entries: Vec<ScopeEntry<'db>>,
}

impl<'db> ScopeData<'db> {
    fn from_parent(parent: ScopeId<'db>) -> Self {
        Self {
            parent: Some(parent),
            entries: Default::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::{path::PathBuf, sync::Arc};

    use salsa::Setter as _;

    use crate::{
        def::{Symbol, hir::Item},
        ide::{self, ExprScopes},
    };

    fn check(source: &str, f: impl FnOnce(Arc<ExprScopes>, &dyn salsa::Database)) {
        let mut db = salsa::DatabaseImpl::default();
        let root = ide::Root::new(&db, vec![], PathBuf::from("test"));
        let file = ide::File::new(
            &db,
            Arc::from(source),
            PathBuf::from("test/src/main.lopa"),
            root,
        );

        root.set_files(&mut db).to(vec![file]);
        let func = file
            .items(&db)
            .items(&db)
            .iter()
            .find_map(|i| {
                if let Item::Function(func) = i {
                    Some(*func)
                } else {
                    None
                }
            })
            .unwrap();

        f(func.expr_scopes(&db).clone(), &db);
    }

    #[test]
    fn fn_scope() {
        check(
            "fn test(value: i32) {
                let x = 20;
                {
                    let z = 12;
                }
            }",
            |scopes, db| {
                let root = scopes.root_scope();
                assert!(
                    scopes
                        .resolve_name_in_scope(root, Symbol::new(db, "x"))
                        .is_some()
                );
                assert!(
                    scopes
                        .resolve_name_in_scope(root, Symbol::new(db, "value"))
                        .is_some()
                );
                assert!(
                    scopes
                        .resolve_name_in_scope(root, Symbol::new(db, "z"))
                        .is_none()
                );

                let second_scope = scopes.scopes.iter().nth(1).unwrap().0;
                assert!(
                    scopes
                        .resolve_name_in_scope(second_scope, Symbol::new(db, "z"))
                        .is_some()
                );
            },
        );
    }
}
