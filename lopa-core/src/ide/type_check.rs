use std::{collections::HashMap, mem::transmute, process::Output};

use crate::{
    def::{
        ExprId, PatId, SymbolList,
        hir::{FnParam, Function, FunctionContents},
        mir::{Type, TypeKind},
    },
    ide::{Diagnostic, File, PathMap},
    ptr::Ptr,
};

#[derive(salsa::SalsaValue, PartialEq)]
pub struct TypeCheckOutput<'db> {
    pattern_ty_map: HashMap<PatId, Type<'db>>,
    expr_ty_map: HashMap<ExprId, Type<'db>>,
    pub diagnostics: Vec<Diagnostic>,
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    fn type_check(self, db: &'db dyn salsa::Database) -> TypeCheckOutput<'db> {
        let mut ctx = TypeCheckCtx {
            db,
            func: self,
            pattern_ty_map: Default::default(),
            expr_ty_map: Default::default(),
            diagnostics: Default::default(),
        };
        ctx.type_check_fn();
        TypeCheckOutput {
            pattern_ty_map: ctx.pattern_ty_map,
            expr_ty_map: ctx.expr_ty_map,
            diagnostics: ctx.diagnostics,
        }
    }
}

//NOTE: currently this only typechecks functions
//TODO: make sure all Bodies are supported, including those attached to fields
struct TypeCheckCtx<'db> {
    db: &'db dyn salsa::Database,
    func: Function<'db>,
    //TODO: replace with ArenaMap if possible
    pattern_ty_map: HashMap<PatId, Type<'db>>,
    expr_ty_map: HashMap<ExprId, Type<'db>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'db> TypeCheckCtx<'db> {
    fn type_check_fn(&mut self) {
        for param in self.func.contents(self.db).params.params(self.db) {
            self.type_check_param(param);
        }
    }

    fn type_check_param(&mut self, param: &FnParam<'db>) {
        match param.kind(self.db) {
            crate::def::hir::FnParamKind::Pat { ty, pat, expr } => {
                if let Some(ty) = ty
                    && let Some(pat) = pat
                {
                    //TODO: verify expression type
                    //Also, checking that the pattern conforms the type needs to happen here
                    // self.pattern_ty_map.insert(pat.id(self.db), ty)
                }
            }
            crate::def::hir::FnParamKind::SelfParam => {}
        }
    }

    fn pat_ty(&self, pat: PatId) -> Option<Type<'db>> {
        self.pattern_ty_map.get(&pat).cloned()
    }

    fn exrp_ty(&self, expr: ExprId) -> Option<Type<'db>> {
        self.expr_ty_map.get(&expr).cloned()
    }
}

fn path_type<'db>(
    db: &'db dyn salsa::Database,
    path: SymbolList,
    path_map: &PathMap<'db>,
) -> Result<Type<'db>, Diagnostic> {
    // let Some(item) = path_map.items.get(&path) else {
    //     return Err(Diagnostic {
    //         message: format!("cannot find type `{}` in this scope"),
    //         location: todo!(),
    //         kind: todo!(),
    //     });
    // };
    todo!()
}

// impl std::ops::Index<ExprId> for TypeCheckCtx<'_> {
//     type Output = Type<'db>;
//
//     fn index(&self, index: ExprId) -> &Self::Output {
//         self.expr_ty_map.get(&index).unwrap()
//     }
// }

// impl<'db> std::ops::Index<PatId> for TypeCheckCtx<'db> {
//     type Output = Type<'db>;
//
//     fn index(&self, index: PatId) -> &Self::Output {
//         self.pattern_ty_map.get(&index).unwrap()
//     }
// }
