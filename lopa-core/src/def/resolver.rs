use std::sync::Arc;

use indexmap::map::Entry;
use itertools::Itertools;
use ustr::Ustr;

use crate::{
    def::{
        self,
        ir::{self, Local},
        scope::{self, ScopeId},
    },
    ustr_hash::UstrIndexMap,
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

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
struct Scope<'db> {
    owner: ir::Function<'db>,
    expr_scopes: Arc<scope::ExprScopes>,
    scope_id: scope::ScopeId,
}

#[derive(Default)]
pub struct ScopeNames<'db> {
    names: UstrIndexMap<ResolveResult<'db>>,
}

impl<'db> ScopeNames<'db> {
    fn add(&mut self, name: &Ustr, def: ResolveResult<'db>) {
        match self.names.entry(name.clone().into()) {
            Entry::Occupied(_) => {}
            Entry::Vacant(entry) => {
                entry.insert(def);
            }
        }
        // //TODO: should this only insert on vacant entries?
        // self.names.insert(name.precomputed_hash(), def);
    }
}

#[derive(Debug)]
pub enum ResolveResult<'db> {
    Local(ir::Local<'db>),
    Function(ir::Function<'db>),
}

impl<'db> Resolver<'db> {
    pub fn names_in_scope(&'db self) -> UstrIndexMap<ResolveResult<'db>> {
        let mut map = ScopeNames::default();

        for scope in self.scopes() {
            for expr_scope in scope.expr_scopes.scope_chain(Some(scope.scope_id)) {
                let entries = scope.expr_scopes.entries(expr_scope);
                for entry in entries {
                    if let Some(owner) = self.body_owner() {
                        map.add(
                            &entry.name(),
                            ResolveResult::Local(Local {
                                parent: owner,
                                pattern_id: entry.pattern(),
                            }),
                        );
                    }
                }
            }
        }

        // for (name, file_def) in self.file_scope.

        map.names
    }

    pub fn body_owner(&'db self) -> Option<ir::Function<'db>> {
        self.scopes().next().map(|s| s.owner)
    }

    fn scopes(&self) -> impl Iterator<Item = &Scope> {
        self.scopes.iter().rev()
    }

    fn push_expr_scope(
        self,
        owner: ir::Function<'db>,
        expr_scopes: Arc<scope::ExprScopes>,
        scope_id: scope::ScopeId,
    ) -> Self {
        self.push_scope(Scope {
            owner,
            expr_scopes,
            scope_id,
        })
    }
    fn push_scope(mut self, scope: Scope<'db>) -> Self {
        self.scopes.push(scope);
        self
    }
}
