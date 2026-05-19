use std::sync::Arc;

use itertools::Itertools;
use ustr::Ustr;

use crate::def::{
    self, ir,
    scope::{self, ScopeId},
};

#[derive(PartialEq, Eq, Clone, salsa::Update)]
pub struct Resolver<'db> {
    scopes: Vec<Scope<'db>>,
    file_scope: scope::FileScope<'db>,
}

#[salsa::tracked(returns(ref))]
pub fn resolver_for_scope<'db>(
    db: &'db dyn salsa::Database,
    owner: ir::Function<'db>,
    scope_id: Option<ScopeId>,
) -> Resolver<'db> {
    let file_scope = scope::file_scope(db, owner.file(db));
    let scopes = scope::expr_scopes(db, owner);
    let scope_chain = scopes.scope_chain(scope_id).collect_vec();
    let mut resolver = Resolver {
        scopes: Vec::with_capacity(scope_chain.len()),
        file_scope: file_scope.clone(),
    };

    for scope in scope_chain.into_iter().rev() {
        resolver = resolver.push_expr_scope(owner, scopes.clone(), scope);
    }

    resolver
}

impl<'db> Resolver<'db> {
    fn push_expr_scope(
        self,
        owner: ir::Function<'db>,
        expr_scopes: Arc<scope::ExprScopes>,
        scope_id: scope::ScopeId,
    ) -> Resolver {
        self.push_scope(Scope {
            owner,
            expr_scopes,
            scope_id,
        })
    }
    fn push_scope(mut self, scope: Scope<'db>) -> Resolver {
        self.scopes.push(scope);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
struct Scope<'db> {
    owner: ir::Function<'db>,
    expr_scopes: Arc<scope::ExprScopes>,
    scope_id: scope::ScopeId,
}

#[derive(Default)]
pub struct ScopeNames<'db> {
    names: indexmap::IndexMap<u64, ResolveResult<'db>>,
}

impl<'db> ScopeNames<'db> {
    fn add(&mut self, name: &Ustr, def: ResolveResult<'db>) {
        //TODO: should this only insert on vacant entries?
        self.names.insert(name.precomputed_hash(), def);
    }
}

#[derive(Debug)]
pub enum ResolveResult<'db> {
    Local(ir::Local<'db>),
    Function(ir::Function<'db>),
}
