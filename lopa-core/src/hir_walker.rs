use crate::def::hir;

#[allow(unused_variables)]
pub trait HirWalker<'db> {
    type Context: Sized;

    fn walk_item(
        &mut self,
        db: &'db dyn salsa::Database,
        item: hir::Item<'db>,
        ctx: &mut Self::Context,
    ) {
        walk_item(db, self, item, ctx);
    }
    fn walk_use_tree(
        &mut self,
        db: &'db dyn salsa::Database,
        use_tree: hir::UseTree<'db>,
        ctx: &mut Self::Context,
    ) {
        walk_use_tree(db, self, use_tree, ctx);
    }

    fn walk_expr(
        &mut self,
        db: &'db dyn salsa::Database,
        expr: hir::Expr<'db>,
        ctx: &mut Self::Context,
    ) {
        walk_expr(db, self, expr, ctx);
    }
    fn walk_stmt(
        &mut self,
        db: &'db dyn salsa::Database,
        stmt: hir::Stmt<'db>,
        ctx: &mut Self::Context,
    ) {
        walk_stmt(db, self, stmt, ctx);
    }
    fn walk_pat(
        &mut self,
        db: &'db dyn salsa::Database,
        pat: hir::Pat<'db>,
        ctx: &mut Self::Context,
    ) {
        walk_pat(db, self, pat, ctx);
    }
    fn walk_type_expr(
        &mut self,
        db: &'db dyn salsa::Database,
        type_expr: hir::TypeExpr<'db>,
        ctx: &mut Self::Context,
    ) {
        walk_type_expr(db, self, type_expr, ctx);
    }
    fn walk_item_type_expr(
        &mut self,
        db: &'db dyn salsa::Database,
        item_type_expr: hir::ItemTypeExpr<'db>,
        ctx: &mut Self::Context,
    ) {
        walk_item_type_expr(db, self, item_type_expr, ctx);
    }
    fn walk_member(
        &mut self,
        db: &'db dyn salsa::Database,
        member: hir::Member<'db>,
        ctx: &mut Self::Context,
    ) {
        walk_member(db, self, member, ctx);
    }
}

pub fn walk_item<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    item: hir::Item<'db>,
    ctx: &mut W::Context,
) {
    match item {
        hir::Item::Function(item) => {
            let contents = item.contents(db);
            for param in contents.params.params(db).iter() {
                match param.kind(db) {
                    hir::FnParamKind::SelfParam => {}
                    hir::FnParamKind::Pat { ty, pat, expr } => {
                        if let Some(ty) = ty {
                            walker.walk_type_expr(db, ty, ctx);
                        }
                        if let Some(pat) = pat {
                            walker.walk_pat(db, pat, ctx);
                        }
                        if let Some(expr) = expr {
                            walker.walk_expr(db, expr, ctx);
                        }
                    }
                }
            }
            if let Some(body_expr) = contents.body_expr {
                walker.walk_expr(db, body_expr, ctx);
            }
        }
        hir::Item::Struct(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walker.walk_member(db, *member, ctx);
            }
        }
        hir::Item::Class(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walker.walk_member(db, *member, ctx);
            }
        }
        hir::Item::Trait(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walker.walk_member(db, *member, ctx);
            }
        }
        hir::Item::Enum(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walker.walk_member(db, *member, ctx);
            }
        }
        hir::Item::Use(item) => {
            if let Some(use_tree) = item.use_tree(db) {
                walker.walk_use_tree(db, use_tree, ctx);
            }
        }
        hir::Item::Module(item) => {
            for item in item.items(db).items(db) {
                walker.walk_item(db, *item, ctx);
            }
        }
        hir::Item::Impl(item) => {
            for func in item.contents(db).fns.items(db).iter() {
                walker.walk_item(db, hir::Item::Function(*func), ctx);
            }
        }
    }
}
pub fn walk_use_tree<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    use_tree: hir::UseTree<'db>,
    ctx: &mut W::Context,
) {
    match use_tree.kind(db) {
        hir::UseTreeKind::Path { use_tree, .. }
        | hir::UseTreeKind::Super { use_tree }
        | hir::UseTreeKind::Root { use_tree } => walker.walk_use_tree(db, use_tree, ctx),
        hir::UseTreeKind::TreeList(use_tree_list) => {
            for use_tree in use_tree_list.items(db) {
                walker.walk_use_tree(db, *use_tree, ctx);
            }
        }
        hir::UseTreeKind::Name(_) | hir::UseTreeKind::SelfUse | hir::UseTreeKind::Global => {}
    }
}

pub fn walk_member<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    member: hir::Member<'db>,
    ctx: &mut W::Context,
) {
    match member.kind(db) {
        hir::MemberKind::Field(field) => {
            if let Some(ty) = field.ty(db) {
                walker.walk_item_type_expr(db, ty, ctx);
            }
            if let Some(default) = field.default_value(db) {
                walker.walk_expr(db, default, ctx);
            }
        }
        hir::MemberKind::Function(function) => {
            walker.walk_item(db, hir::Item::Function(function), ctx);
        }
        hir::MemberKind::Variant(variant) => {
            if let Some(ty) = variant.ty(db) {
                walker.walk_item_type_expr(db, *ty, ctx);
            }
            if let Some(expr) = variant.expr(db) {
                walker.walk_expr(db, *expr, ctx);
            }
        }
    }
}

pub fn walk_item_type_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    item_type_expr: hir::ItemTypeExpr<'db>,
    ctx: &mut W::Context,
) {
    match item_type_expr.kind(db) {
        hir::ItemTypeExprKind::TypeExpr(type_expr) => {
            walker.walk_type_expr(db, type_expr, ctx);
        }
        hir::ItemTypeExprKind::Struct(item) => {
            walker.walk_item(db, hir::Item::Struct(item), ctx);
        }
        hir::ItemTypeExprKind::Enum(item) => {
            walker.walk_item(db, hir::Item::Enum(item), ctx);
        }
        hir::ItemTypeExprKind::Class(item) => {
            walker.walk_item(db, hir::Item::Class(item), ctx);
        }
    }
}

pub fn walk_type_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    type_expr: hir::TypeExpr<'db>,
    ctx: &mut W::Context,
) {
    match type_expr.kind(db) {
        hir::TypeExprKind::Tuple(type_expr_list) | hir::TypeExprKind::Dyn(type_expr_list) => {
            for ty in type_expr_list.types(db) {
                walker.walk_type_expr(db, *ty, ctx);
            }
        }
        hir::TypeExprKind::Nilable(type_expr) | hir::TypeExprKind::Paren(type_expr) => {
            walker.walk_type_expr(db, type_expr, ctx);
        }
        hir::TypeExprKind::Fn { params, output } => {
            for param in params.params(db).iter() {
                if let Some(ty) = param.ty(db) {
                    walker.walk_type_expr(db, ty, ctx);
                }
            }
            if let Some(output) = output {
                walker.walk_type_expr(db, output, ctx);
            }
        }
        hir::TypeExprKind::Any
        | hir::TypeExprKind::Unit
        | hir::TypeExprKind::Never
        | hir::TypeExprKind::Lit(_)
        | hir::TypeExprKind::Path(_)
        | hir::TypeExprKind::SelfTy => {}
    }
}

pub fn walk_pat<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    pat: hir::Pat<'db>,
    ctx: &mut W::Context,
) {
    match pat.kind(db) {
        hir::PatKind::Paren(pat) => {
            walker.walk_pat(db, pat, ctx);
        }
        hir::PatKind::Tuple(pat_list) => {
            for pat in pat_list.patterns(db).iter() {
                walker.walk_pat(db, *pat, ctx);
            }
        }
        hir::PatKind::Wildcard | hir::PatKind::Name(_) | hir::PatKind::Path(_) => {}
    }
}
pub fn walk_stmt<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    stmt: hir::Stmt<'db>,
    ctx: &mut W::Context,
) {
    match stmt.kind(db) {
        hir::StmtKind::Let { pat, ty, expr } => {
            walker.walk_expr(db, expr, ctx);
            walker.walk_pat(db, pat, ctx);
            if let Some(ty) = ty {
                walker.walk_type_expr(db, ty, ctx);
            }
        }
        hir::StmtKind::Expr { expr, .. } => {
            walker.walk_expr(db, expr, ctx);
        }
    }
}

pub fn walk_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    expr: hir::Expr<'db>,
    ctx: &mut W::Context,
) {
    match expr.kind(db) {
        hir::ExprKind::As { expr, ty } => {
            walker.walk_expr(db, expr, ctx);
            walker.walk_type_expr(db, ty, ctx);
        }
        hir::ExprKind::Is { expr, pat } => {
            walker.walk_expr(db, expr, ctx);
            walker.walk_pat(db, pat, ctx);
        }
        hir::ExprKind::IsNot { expr, pat } => {
            walker.walk_expr(db, expr, ctx);
            walker.walk_pat(db, pat, ctx);
        }
        hir::ExprKind::Closure {
            params,
            body,
            output,
        } => {
            for param in params.params(db) {
                walker.walk_pat(db, param.pat(db), ctx);
                if let Some(ty) = param.ty(db) {
                    walker.walk_type_expr(db, ty, ctx);
                }
                walker.walk_expr(db, body, ctx);
            }
            if let Some(output) = output {
                walker.walk_type_expr(db, output, ctx);
            }
        }
        hir::ExprKind::Field { expr, .. } => {
            walker.walk_expr(db, expr, ctx);
        }
        hir::ExprKind::Method { expr, args, .. } => {
            walker.walk_expr(db, expr, ctx);
            walk_args(db, walker, args, ctx);
        }
        hir::ExprKind::Record { fields, .. } => {
            for field in fields.fields(db).iter() {
                walker.walk_expr(db, field.expr(db), ctx);
            }
        }
        hir::ExprKind::Binary { lhs, rhs, .. } => {
            walker.walk_expr(db, lhs, ctx);
            walker.walk_expr(db, rhs, ctx);
        }
        hir::ExprKind::Unary { expr, .. } => {
            walker.walk_expr(db, expr, ctx);
        }
        hir::ExprKind::Index { base, index } => {
            walker.walk_expr(db, base, ctx);
            walker.walk_expr(db, index, ctx);
        }
        hir::ExprKind::Call { func, args } => {
            walker.walk_expr(db, func, ctx);
            walk_args(db, walker, args, ctx);
        }
        hir::ExprKind::Block { stmts } => {
            for stmt in stmts.stmts(db).iter() {
                walker.walk_stmt(db, *stmt, ctx);
            }
        }
        hir::ExprKind::Paren(expr) | hir::ExprKind::Return(expr) => {
            walker.walk_expr(db, expr, ctx);
        }
        hir::ExprKind::If(if_expr) => {
            walk_if_expr(db, walker, if_expr, ctx);
        }
        hir::ExprKind::Loop { block } => {
            walker.walk_expr(db, block, ctx);
        }
        hir::ExprKind::While { cond, block } => {
            walker.walk_expr(db, cond, ctx);
            walker.walk_expr(db, block, ctx);
        }
        hir::ExprKind::For {
            loop_expr,
            iterable,
            block,
        } => {
            walker.walk_expr(db, loop_expr, ctx);
            walker.walk_expr(db, iterable, ctx);
            walker.walk_expr(db, block, ctx);
        }
        hir::ExprKind::Tuple { exprs } => {
            for expr in exprs.exprs(db).iter() {
                walker.walk_expr(db, *expr, ctx);
            }
        }
        hir::ExprKind::Unit
        | hir::ExprKind::SelfExpr
        | hir::ExprKind::Lit(_)
        | hir::ExprKind::Path(_) => {}
    }
}

fn walk_args<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    args: hir::Args<'db>,
    ctx: &mut W::Context,
) {
    for arg in args.args(db).iter() {
        match arg.kind(db) {
            hir::ArgKind::Labeled { value, .. } | hir::ArgKind::NonLabeled { value } => {
                walker.walk_expr(db, value, ctx);
            }
        }
    }
}

pub fn walk_if_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    if_expr: hir::IfExpr<'db>,
    ctx: &mut W::Context,
) {
    walker.walk_expr(db, if_expr.cond(db), ctx);
    walker.walk_expr(db, if_expr.if_branch(db), ctx);
    if let Some(else_branch) = if_expr.else_branch(db) {
        match else_branch {
            hir::ElseBranch::Block(expr) => walker.walk_expr(db, expr, ctx),
            hir::ElseBranch::If(if_expr) => walk_if_expr(db, walker, if_expr, ctx),
        }
    }
}
