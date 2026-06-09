use salsa::Accumulator;
use ustr::Ustr;

use crate::{
    def::{
        ir::{self, ExprId, Local, ModuleDef},
        scope::{self, ScopeId},
    },
    ide::{
        self,
        diagnostics::{Diagnostic, DiagnosticKind},
    },
    ustr_hash::UstrIndexMap,
};
//
// #[derive(PartialEq, Eq, Clone, salsa::Update)]
// pub struct Resolver<'db> {
//     scopes: Vec<Scope<'db>>,
//     file_scope: scope::FileScope<'db>,
// }
//
// #[salsa::tracked(returns(ref))]
// pub fn resolver_for_top_level<'db>(db: &'db dyn salsa::Database, file: ide::File) -> Resolver<'db> {
//     let scopes = scope::file_scope(db, file);
//     Resolver {
//         scopes: vec![],
//         file_scope: scopes.clone(),
//     }
// }
//
// #[salsa::tracked(returns(ref))]
// pub fn resolver_for_expr<'db>(
//     db: &'db dyn salsa::Database,
//     owner: ir::Function<'db>,
//     expr_id: ExprId,
// ) -> Resolver<'db> {
//     let scopes = scope::expr_scopes(db, owner);
//     resolver_for_scope(db, owner, scopes.scope_for_expr(expr_id)).clone()
// }
//
// #[salsa::tracked(returns(ref))]
// pub fn resolver_for_scope<'db>(
//     db: &'db dyn salsa::Database,
//     owner: ir::Function<'db>,
//     scope_id: Option<ScopeId>,
// ) -> Resolver<'db> {
//     let file_scope = scope::file_scope(db, owner.file(db));
//     let scopes = scope::expr_scopes(db, owner);
//     let scope_chain = scopes.scope_chain(scope_id).collect_vec();
//
//     let mut resolver = Resolver {
//         scopes: Vec::with_capacity(scope_chain.len()),
//         file_scope: file_scope.clone(),
//     };
//     for scope in scope_chain.into_iter().rev() {
//         resolver = resolver.push_expr_scope(owner, scopes.clone(), scope);
//     }
//
//     resolver
// }
//
// #[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
// struct Scope<'db> {
//     owner: ir::Function<'db>,
//     expr_scopes: Arc<scope::ExprScopes>,
//     scope_id: scope::ScopeId,
// }
//
#[derive(Default)]
pub struct ScopeNames<'db> {
    names: UstrIndexMap<ResolveResult<'db>>,
}
//
// impl<'db> ScopeNames<'db> {
//     fn add(&mut self, name: &Ustr, def: ResolveResult<'db>) {
//         match self.names.entry((*name).into()) {
//             Entry::Occupied(_) => {}
//             Entry::Vacant(entry) => {
//                 entry.insert(def);
//             }
//         }
//         // //TODO: should this only insert on vacant entries?
//         // self.names.insert(name.precomputed_hash(), def);
//     }
// }
//
#[derive(Debug)]
pub enum ResolveResult<'db> {
    Local(ir::Local<'db>),
    Function(ir::Function<'db>),
    Struct(ir::Struct<'db>),
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, salsa::Update)]
pub enum ResolveItemResult<'db> {
    Type(ir::ModuleDef<'db>),
    Value(ir::ModuleDef<'db>),
    Both {
        ty: ir::ModuleDef<'db>,
        value: ir::ModuleDef<'db>,
    },
}

// impl<'db> Resolver<'db> {
//     pub fn names_in_scope(&self) -> UstrIndexMap<ResolveResult<'_>> {
//         let mut map = ScopeNames::default();
//
//         for scope in self.scopes() {
//             for expr_scope in scope.expr_scopes.scope_chain(Some(scope.scope_id)) {
//                 let entries = scope.expr_scopes.entries(expr_scope);
//                 for entry in entries {
//                     if let Some(owner) = self.body_owner() {
//                         map.add(
//                             &entry.name(),
//                             ResolveResult::Local(Local {
//                                 parent: owner,
//                                 pattern_id: entry.pattern(),
//                             }),
//                         );
//                     }
//                 }
//             }
//         }
//
//         //TODO: finish
//         // for (name, file_def) in self.file_scope.values() {
//         //     match file_def {
//         //         ir::FileDef::Function(function) => {
//         //             map.add(&name.0, ResolveResult::Function(*function));
//         //         }
//         //     }
//         // }
//
//         map.names
//     }
//
//     pub fn resolve_name(&self, name: &Ustr) -> Option<ResolveResult<'_>> {
//         for scope in self.scopes() {
//             let entry = scope
//                 .expr_scopes
//                 .resolve_name_in_scope(scope.scope_id, name);
//
//             if let (Some(entry), Some(owner)) = (entry, self.body_owner()) {
//                 return Some(ResolveResult::Local(Local {
//                     parent: owner,
//                     pattern_id: entry.pattern(),
//                 }));
//             }
//         }
//
//         //TODO: finish
//         // if let Some(result) = self.file_scope.resolve_name(name) {
//         //     match result {
//         //         ir::FileDef::Function(function) => return Some(ResolveResult::Function(*function)),
//         //     }
//         // }
//
//         None
//     }
//
//     pub fn body_owner(&self) -> Option<ir::Function<'_>> {
//         self.scopes().next().map(|s| s.owner)
//     }
//
//     fn scopes(&self) -> impl Iterator<Item = &Scope<'_>> {
//         self.scopes.iter().rev()
//     }
//
//     fn push_expr_scope(
//         self,
//         owner: ir::Function<'db>,
//         expr_scopes: Arc<scope::ExprScopes>,
//         scope_id: scope::ScopeId,
//     ) -> Self {
//         self.push_scope(Scope {
//             owner,
//             expr_scopes,
//             scope_id,
//         })
//     }
//     fn push_scope(mut self, scope: Scope<'db>) -> Self {
//         self.scopes.push(scope);
//         self
//     }
// }
//
// #[cfg(test)]
// mod test {
//     use std::sync::{Arc, RwLock};
//
//     use salsa::{Database, DatabaseImpl};
//
//     use crate::ide::{self, FileContent};
//
//     #[test]
//     fn names_in_scope() {
//         //TODO: write an acutal test
//         DatabaseImpl::default().attach(|db| {
//             let input = ide::File::new(
//                 db,
//                 Arc::new(RwLock::new(FileContent::new(String::from(
//                     "fn main() {
//                     let x = 1;
//                     if true {
//                         let y = 1;
//                     }
//                 }",
//                 )))),
//             );
//             let ir = ide::lower_file(db, input);
//             dbg!(ir.functions(db));
//         });
//     }
// }
//
#[salsa::tracked]
pub fn resolve_item_name<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    name: Ustr,
) -> Option<ResolveItemResult<'db>> {
    let module_scope = scope::module_scope(db, file);
    let value = module_scope.value_item(&name).cloned();
    let ty = module_scope.type_item(&name).cloned();
    Some(match (value, ty) {
        (Some(value), Some(ty)) => ResolveItemResult::Both { ty, value },
        (Some(value), None) => ResolveItemResult::Value(value),
        (None, Some(ty)) => ResolveItemResult::Type(ty),
        _ => return None,
    })
}
#[salsa::tracked]
pub fn resolve_path<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    path: ir::Path,
) -> Option<ResolveItemResult<'db>> {
    fn resolve_path_inner<'db>(
        db: &'db dyn salsa::Database,
        file: ide::File,
        path: ir::Path,
    ) -> Option<ResolveItemResult<'db>> {
        let first = *path.0.first()?;
        let mut current_item = match first.as_str() {
            "root" => ResolveItemResult::Type(ModuleDef::Module(ide::root_module(
                db,
                file.source_root(db),
            )?)),
            _ => resolve_item_name(db, file, *path.0.first()?).or_else(|| {
                let scope = scope::module_scope(db, file);
                for global_path in scope.global_imports() {
                    let mut global_path = global_path.clone();
                    global_path.0.push(*path.0.first()?);
                    if path == global_path {
                        return None;
                    }
                    let Some(output) = resolve_path(db, file, global_path.clone()) else {
                        continue;
                    };
                    return Some(output);
                }
                None
            })?,
        };
        for (id, segment) in path.0.iter().skip(1).enumerate() {
            match current_item.clone() {
                ResolveItemResult::Type(module_def) => match module_def {
                    ModuleDef::Struct(strct) => {
                        if id == path.0.len() - 1 {
                            return Some(ResolveItemResult::Type(ModuleDef::Struct(strct)));
                        }
                        return None;
                    }
                    ModuleDef::Module(file) => {
                        if id == path.0.len() - 1 {
                            return Some(ResolveItemResult::Type(ModuleDef::Module(file)));
                        }
                        if segment == "self" {
                            continue;
                        }
                        let mut module_path = ide::module_path(db, file);
                        module_path.0.push(*segment);
                        //TODO: public/private imports
                        current_item = resolve_path(db, file, ir::Path(vec![*segment]))?;
                    }
                    _ => unreachable!(),
                },
                ResolveItemResult::Value(module_def) => match module_def {
                    ModuleDef::Function(func) => {
                        if id == path.0.len() - 1 {
                            return Some(ResolveItemResult::Value(ModuleDef::Function(func)));
                        }
                        return None;
                    }
                    _ => unreachable!(),
                },
                ResolveItemResult::Both { ty, value } => {
                    let ty = match ty {
                        ModuleDef::Struct(strct) => {
                            if id == path.0.len() - 1 {
                                Some(ModuleDef::Struct(strct))
                            } else {
                                None
                            }
                        }
                        ModuleDef::Module(file) => {
                            if id == path.0.len() - 1 {
                                Some(ModuleDef::Module(file))
                            } else {
                                if segment == "self" {
                                    continue;
                                }
                                let mut module_path = ide::module_path(db, file);
                                module_path.0.push(*segment);
                                //TODO: public/private imports
                                current_item = resolve_path(db, file, ir::Path(vec![*segment]))?;
                                None
                            }
                        }
                        _ => unreachable!(),
                    };
                    let value = match value.clone() {
                        ModuleDef::Function(func) => {
                            if id == path.0.len() - 1 {
                                Some(ModuleDef::Function(func))
                            } else {
                                None
                            }
                        }
                        _ => unreachable!(),
                    };

                    match (ty, value) {
                        (Some(ty), Some(value)) => {
                            return Some(ResolveItemResult::Both { ty, value });
                        }
                        (Some(ty), None) => return Some(ResolveItemResult::Type(ty)),
                        (None, Some(value)) => return Some(ResolveItemResult::Value(value)),
                        _ => {}
                    }
                }
            }
        }
        Some(current_item)
    }

    let module_scope = scope::module_scope(db, file);
    let first = path.0.first()?;
    if let Some(scope::ScopeName { path: outer, .. }) = module_scope
        .type_scope_names(first)
        .or_else(|| module_scope.value_scope_names(first))
    {
        let mut outer = outer.clone();
        outer.0.remove(outer.0.len() - 1);
        outer.0.append(&mut path.0.clone());
        if outer == path {
            resolve_path_inner(db, file, path)
        } else {
            resolve_path(db, file, outer)
        }
    } else {
        resolve_path_inner(db, file, path)
    }
}

pub fn resolve_name_for_expr<'db>(
    db: &'db dyn salsa::Database,
    expr: ExprId,
    func: ir::Function<'db>,
    name: &Ustr,
) -> Option<ResolveResult<'db>> {
    let scopes = scope::expr_scopes(db, func);
    let expr_scope = scopes.scope_for_expr(expr)?;

    if let Some(entry) = scopes.resolve_name_in_scope(expr_scope, &name) {
        return Some(ResolveResult::Local(Local {
            parent: func,
            pattern_id: entry.pattern(),
        }));
    }
    let module_scope = scope::module_scope(db, func.file(db));

    if let Some(result) = module_scope.value_item(name) {
        match result {
            ir::ModuleDef::Function(function) => {
                return Some(ResolveResult::Function(*function));
            }
            _ => unreachable!(),
        }
    }
    None
}

#[salsa::tracked]
pub fn visible_module_items<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
) -> UstrIndexMap<ResolveItemResult<'db>> {
    let mut items = UstrIndexMap::<ResolveItemResult<'db>>::default();
    let try_insert = |name: Ustr, text_range: rowan::TextRange, item: ResolveItemResult<'db>| {
        if items.insert(name.into(), item).is_some() {
            Diagnostic::new(
                text_range,
                DiagnosticKind::ModuleError,
                format!("the name `{}` is defined multiple times", name),
            )
            .accumulate(db);
        }
    };

    let module_scope = scope::module_scope(db, file);
    items
}

#[salsa::tracked]
pub fn resolve_name<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    name: Ustr,
) -> Option<ResolveItemResult<'db>> {
    None
}

// pub fn resolve_name(&self, name: &Ustr) -> Option<ResolveResult<'_>> {
//     for scope in self.scopes() {
//         let entry = scope
//             .expr_scopes
//             .resolve_name_in_scope(scope.scope_id, name);
//
//         if let (Some(entry), Some(owner)) = (entry, self.body_owner()) {
//             return Some(ResolveResult::Local(Local {
//                 parent: owner,
//                 pattern_id: entry.pattern(),
//             }));
//         }
//     }
//
//     //TODO: finish
//     // if let Some(result) = self.file_scope.resolve_name(name) {
//     //     match result {
//     //         ir::FileDef::Function(function) => return Some(ResolveResult::Function(*function)),
//     //     }
//     // }
//
//     None
// }
//
