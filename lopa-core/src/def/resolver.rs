use rowan::ast::AstNode;
use salsa::Accumulator;
use ustr::Ustr;

use crate::{
    def::{
        body,
        ir::{self, ExprId, Local, ModuleDef},
        scope::{self, ScopeId},
    },
    ide::{
        self,
        diagnostics::{Diagnostic, DiagnosticKind},
    },
};

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
                    ModuleDef::Enum(enum_item) => {
                        if id == path.0.len() - 1 {
                            return Some(ResolveItemResult::Type(ModuleDef::Enum(enum_item)));
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
                        ModuleDef::Enum(enum_item) => {
                            if id == path.0.len() - 1 {
                                return Some(ResolveItemResult::Type(ModuleDef::Enum(enum_item)));
                            }
                            return None;
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

pub fn resolve_path_for_expr<'db>(
    db: &'db dyn salsa::Database,
    expr: ExprId,
    func: ir::Function<'db>,
    path: &ir::Path,
) -> Option<ResolveResult<'db>> {
    let scopes = scope::expr_scopes(db, func);
    let expr_scope = scopes.scope_for_expr(expr)?;

    if let [name] = path.0.as_slice()
        && let Some(entry) = scopes.resolve_name_in_scope(expr_scope, name)
    {
        return Some(ResolveResult::Local(Local {
            parent: func,
            pattern_id: entry.pattern(),
        }));
    }

    let Some(result) = resolve_path(db, func.file(db), path.clone()) else {
        Diagnostic::new(
            body::expr_range(db, func, expr)?,
            DiagnosticKind::TypeError,
            format!(
                "cannot find value `{}` in this scope",
                body::expr_text(db, func, expr)?
            ),
        )
        .accumulate(db);
        return None;
    };

    match result {
        ResolveItemResult::Value(value) | ResolveItemResult::Both { value, .. } => match value {
            ModuleDef::Function(function) => Some(ResolveResult::Function(function)),
            ModuleDef::Struct(_) => unreachable!(),
            ModuleDef::Enum(_) => unreachable!(),
            ModuleDef::Module(_) => unreachable!(),
        },
        _ => {
            Diagnostic::new(
                body::expr_range(db, func, expr)?,
                DiagnosticKind::TypeError,
                format!(
                    "cannot find value `{}` in this scope",
                    body::expr_text(db, func, expr)?
                ),
            )
            .accumulate(db);
            None
        }
    }
}

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
