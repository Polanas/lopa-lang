use itertools::Itertools;
use notify_rust::Notification;
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
    parsing::ast,
};

#[derive(Debug)]
pub enum ResolveResult<'db> {
    Local(ir::Local<'db>),
    Function(ir::Type<'db>),
    Struct(ir::Type<'db>),
    Enum(ir::Type<'db>),
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

fn resolve_path_cycle_result<'db>(
    _db: &'db dyn salsa::Database,
    _id: salsa::Id,
    _file: ide::File,
    _path: ir::Path,
) -> Option<ResolveItemResult<'db>> {
    None
}

pub fn resolve_item_type_expr<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    ty: ast::ItemTypeExpr,
    generics: Option<&ir::Generics<'db>>,
    self_ty: Option<&ir::Type<'db>>,
) -> ir::Type<'db> {
    match ty {
        ast::ItemTypeExpr::StructItemType(struct_item_type) => {
            let Some(struct_name) = struct_item_type
                .struct_item()
                .and_then(|e| e.name())
                .and_then(|e| e.text())
            else {
                return ir::Type::Unknown;
            };
            let Some(result) = resolve_path_item(db, file, ir::Path(vec![struct_name])) else {
                return ir::Type::Unknown;
            };
            match result {
                ResolveItemResult::Type(ty) | ResolveItemResult::Both { ty, .. }
                    if let ir::ModuleDef::Struct(struct_item) = ty =>
                {
                    struct_item.generic_type(db).clone()
                }
                _ => ir::Type::Unknown,
            }
        }
        ast::ItemTypeExpr::EnumItemType(enum_item_type) => {
            let Some(enum_name) = enum_item_type
                .enum_item()
                .and_then(|e| e.name())
                .and_then(|e| e.text())
            else {
                return ir::Type::Unknown;
            };
            let Some(result) = resolve_path_item(db, file, ir::Path(vec![enum_name])) else {
                return ir::Type::Unknown;
            };
            match result {
                ResolveItemResult::Type(ty) | ResolveItemResult::Both { ty, .. }
                    if let ir::ModuleDef::Enum(enum_item) = ty =>
                {
                    enum_item.generic_type(db).clone()
                }
                _ => ir::Type::Unknown,
            }
        }
        ast::ItemTypeExpr::ItemType(item_type) => {
            let Some(ty) = item_type.ty() else {
                return ir::Type::Unknown;
            };
            resolve_type_expr(db, file, ty, generics, self_ty)
        }
    }
}

pub fn resolve_type_expr<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    ty: ast::TypeExpr,
    generics: Option<&ir::Generics<'db>>,
    self_ty: Option<&ir::Type<'db>>,
) -> ir::Type<'db> {
    let range = ty.syntax().text_range();
    match ty {
        ast::TypeExpr::PathType(path_type) => {
            let Some(path) = path_type.value() else {
                return ir::Type::Unknown;
            };
            resolve_type_path(db, file, path, generics, self_ty)
        }
        ast::TypeExpr::NilableType(nilable_type) => {
            let Some(ty) = nilable_type.ty() else {
                let text = nilable_type.syntax().text().to_string();
                Diagnostic::new(
                    range,
                    DiagnosticKind::TypeError,
                    format!("cannot find type `{}` in this scope", &text),
                )
                .accumulate(db);
                return ir::Type::Unknown;
            };
            ir::Type::Nilable(Box::new(resolve_type_expr(db, file, ty, generics, self_ty)))
        }
        ast::TypeExpr::LitType(lit_type) => {
            let Some(kind) = lit_type.kind() else {
                let text = lit_type.syntax().text().to_string();
                Diagnostic::new(
                    range,
                    DiagnosticKind::TypeError,
                    format!("cannot find type `{}` in this scope", &text),
                )
                .accumulate(db);
                return ir::Type::Unknown;
            };

            ir::Type::Lit(kind)
        }
        ast::TypeExpr::AnyType(_) => ir::Type::Any,
        ast::TypeExpr::UnitType(_) => ir::Type::Unit,
        ast::TypeExpr::FnType(fn_type) => ir::Type::BareFn(ir::BareFn {
            params: fn_type
                .param_list()
                .map(|list| {
                    list.params()
                        .filter_map(|param| {
                            param
                                .ty()
                                .map(|ty| resolve_type_expr(db, file, ty, generics, self_ty))
                                .map(|ty| (ty, param.name()))
                        })
                        .map(|(ty, n)| ir::Param {
                            name: n.and_then(|n| n.text()),
                            ty,
                        })
                        .collect_vec()
                })
                .unwrap_or_default(),
            output: fn_type
                .output()
                .and_then(|o| o.ty())
                .map(|ty| resolve_type_expr(db, file, ty, generics, self_ty))
                .unwrap_or_else(|| ir::Type::Unit)
                .into(),
        }),
        ast::TypeExpr::SelfType(_) => {
            if let Some(owner) = self_ty {
                owner.clone()
            } else {
                Diagnostic::new(
                    ty.syntax().text_range(),
                    DiagnosticKind::TypeError,
                    "cannot find type `Self` in this scope".to_string(),
                )
                .accumulate(db);
                ir::Type::Unknown
            }
        }
        ast::TypeExpr::DynType(dyn_type) => {
            let Some(path) = dyn_type.path() else {
                return ir::Type::Unknown;
            };
            resolve_type_path(db, file, path, generics, self_ty)
        }
    }
}

pub fn resolve_type_path<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    ast_path: ast::Path,
    generics: Option<&ir::Generics<'db>>,
    self_ty: Option<&ir::Type<'db>>,
) -> ir::Type<'db> {
    let path = ir::Path(ast_path.segments().collect_vec());
    if let [first] = path.0.as_slice()
        && let Some(generics) = generics.as_ref()
        && let Some(param) = generics.param(first)
    {
        return ir::Type::Generic(param.name);
    }
    //TODO: account for <>
    let generic_params = ast_path
        .generic_args()
        .map(|args| {
            args.types()
                .map(|ty| resolve_type_expr(db, file, ty, generics, self_ty))
        })
        .map(|args| ir::GenericParams::new(args.collect_vec()))
        .unwrap_or_else(|| ir::GenericParams::default());

    let Some(item) = resolve_path_item(db, file, path.clone()) else {
        let path_text = ast_path.syntax().text().to_string();
        Diagnostic::new(
            ast_path.syntax().text_range(),
            DiagnosticKind::TypeError,
            format!("cannot find type `{}` in this scope", path_text),
        )
        .accumulate(db);
        return ir::Type::Unknown;
    };

    let ty = match item {
        ResolveItemResult::Type(ty) | ResolveItemResult::Both { ty, .. } => match ty {
            ir::ModuleDef::Struct(strct) => ir::Type::Struct(strct, generic_params),
            ir::ModuleDef::Enum(enum_item) => ir::Type::Enum(enum_item, generic_params),
            ir::ModuleDef::Module(module) => {
                Diagnostic::new(
                    ast_path.syntax().text_range(),
                    DiagnosticKind::TypeError,
                    format!(
                        "expected type, got module `{}`",
                        ide::module_name(db, module)
                    ),
                )
                .accumulate(db);
                ir::Type::Unknown
            }
            _ => unreachable!(),
        },
        ResolveItemResult::Value(value) => match value {
            ir::ModuleDef::Function(function) => {
                Diagnostic::new(
                    ast_path.syntax().text_range(),
                    DiagnosticKind::TypeError,
                    format!("expected type, got function `{}`", function.name(db)),
                )
                .accumulate(db);
                ir::Type::Unknown
            }
            _ => unreachable!(),
        },
    };

    ty
}

#[salsa::tracked(cycle_result=resolve_path_cycle_result)]
pub fn resolve_path_item<'db>(
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
                    let Some(output) = resolve_path_item(db, file, global_path.clone()) else {
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
                        let mod_path = ir::Path(vec![*segment]);
                        current_item = resolve_path_item(db, file, mod_path)?;
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
                                let mod_path = ir::Path(vec![*segment]);
                                current_item = resolve_path_item(db, file, mod_path)?;
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
            resolve_path_item(db, file, outer)
        }
    } else {
        resolve_path_inner(db, file, path)
    }
}

pub fn resolve_path_for_expr<'db>(
    db: &'db dyn salsa::Database,
    expr: ExprId,
    func: ir::Function<'db>,
    path: &ir::GenericPath<'db>,
) -> Option<ResolveResult<'db>> {
    let scopes = scope::expr_scopes(db, func);
    let expr_scope = scopes.scope_for_expr(expr)?;

    if let [name] = path.value.as_slice()
        && let Some(entry) = scopes.resolve_name_in_scope(expr_scope, name)
    {
        return Some(ResolveResult::Local(Local {
            parent: func,
            pattern_id: entry.pattern(),
        }));
    }

    let Some(result) = resolve_path_item(db, func.file(db), ir::Path(path.value.clone())) else {
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
            ModuleDef::Function(function) => Some(ResolveResult::Function(ir::Type::Function(
                function,
                path.params.clone(),
            ))),
            ModuleDef::Struct(struct_item) => Some(ResolveResult::Struct(ir::Type::Struct(
                struct_item,
                path.params.clone(),
            ))),
            ModuleDef::Enum(enum_item) => Some(ResolveResult::Enum(ir::Type::Enum(
                enum_item,
                path.params.clone(),
            ))),
            ModuleDef::Module(_) => {
                return None;
            }
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
