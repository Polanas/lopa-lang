use std::{collections::HashMap, mem::transmute};

use crate::{
    def::{
        ExprId, PatId,
        hir::{Function, FunctionContents},
        mir::Type,
    },
    ide::Diagnostic,
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
        let ctx = TypeCheckCtx {
            db,
            func: unsafe { transmute::<Function<'db>, Function<'static>>(self) },
            pattern_ty_map: Ptr::new(Default::default()),
            expr_ty_map: Ptr::new(Default::default()),
            contents: unsafe {
                transmute::<&FunctionContents<'db>, &FunctionContents<'static>>(self.contents(db))
            },
            diagnostics: Default::default(),
        };
        ctx.type_check_fn();
        TypeCheckOutput {
            pattern_ty_map: ctx.pattern_ty_map.try_unwrap().unwrap(),
            expr_ty_map: ctx.expr_ty_map.try_unwrap().unwrap(),
            diagnostics: ctx.diagnostics,
        }
    }
}

//NOTE: currently this only typechecks functions
//TODO: make sure all Bodies are supported, including those attached to fields
struct TypeCheckCtx<'db> {
    db: &'db dyn salsa::Database,
    func: Function<'static>,
    contents: &'db FunctionContents<'static>,
    //TODO: replace with ArenaMap if possible
    //also maybe try finding another solution that doesnt depend on 'static and Ptr
    pattern_ty_map: Ptr<HashMap<PatId, Type<'static>>>,
    expr_ty_map: Ptr<HashMap<ExprId, Type<'static>>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'db> TypeCheckCtx<'db> {
    fn type_check_fn(&self) {}
}

impl std::ops::Index<ExprId> for TypeCheckCtx<'_> {
    type Output = Type<'static>;

    fn index(&self, index: ExprId) -> &Self::Output {
        self.expr_ty_map.get(&index).unwrap()
    }
}

impl std::ops::Index<PatId> for TypeCheckCtx<'_> {
    type Output = Type<'static>;

    fn index(&self, index: PatId) -> &Self::Output {
        self.pattern_ty_map.get(&index).unwrap()
    }
}
