use itertools::Itertools;
use ustr::Ustr;

use crate::{
    def::ir::{self, Function},
    ide::{self, diagnostics::Diagnostic},
    parsing::{ast, parser},
};

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
pub struct FunctionName(pub Ustr);

impl identity_hash::IdentityHashable for FunctionName {}

impl std::hash::Hash for FunctionName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.precomputed_hash());
    }
}

indexmap_hash! {
    FunctionMap<'db>(indexmap::IndexMap<FunctionName, ir::Function<'db>, identity_hash::BuildIdentityHasher<FunctionName>>)
}

#[salsa::tracked]
pub struct IrFile<'db> {
    pub functions: Vec<ir::Function<'db>>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct LowerContext<'db> {
    pub db: &'db dyn salsa::Database,
    pub diagnostics: Vec<Diagnostic>,
    pub functions: Vec<ir::Function<'db>>,
    pub ast_file: ast::File,
    pub file: ide::File,
}

impl<'db> LowerContext<'db> {
    pub fn new(db: &'db dyn salsa::Database, parse: ide::Parse<'db>, file: ide::File) -> Self {
        Self {
            diagnostics: Default::default(),
            functions: Default::default(),
            ast_file: parse.file(db),
            file,
            db,
        }
    }

    pub fn lower(mut self, file: ast::File) -> IrFile<'db> {
        for item in file.items() {
            self.item(item);
        }
        IrFile::new(
            self.db,
            self.functions,
            // self.functions
            //     .into_iter()
            //     .map(|f| (FunctionName(f.name(self.db)), f))
            //     .collect::<indexmap::IndexMap<_, _, identity_hash::BuildIdentityHasher<FunctionName>>>()
            //     .into(),
            self.diagnostics,
        )
    }

    fn item(&mut self, item: ast::Item) {
        match item {
            ast::Item::FnItem(fn_item) => {
                if let Some(item) = self.fn_item(fn_item) {
                    self.functions.push(item);
                }
            }
        };
    }

    fn fn_item(&self, fn_item: ast::FnItem) -> Option<ir::Function<'db>> {
        Some(ir::Function::new(
            self.db,
            fn_item.name()?.text()?,
            fn_item
                .params()?
                .params()
                .filter_map(|p| self.param(p))
                .collect_vec(),
            fn_item
                .output()
                .and_then(|o| o.ty())
                .and_then(|o| self.type_expr(o)),
            ast::AstPtr::new(&fn_item),
            self.file,
        ))
    }

    fn param(&self, param: ast::FnParam) -> Option<ir::FnParam<'db>> {
        Some(ir::FnParam::new(
            self.db,
            param.name()?.text()?,
            self.type_expr(param.ty()?)?,
        ))
    }

    fn type_expr(&self, item: ast::TypeExpr) -> Option<ir::TypeExpr> {
        Some(match item {
            ast::TypeExpr::Name(name) => ir::TypeExpr::NameType(self.name_type(name)?),
            ast::TypeExpr::NilableType(nilable_type) => {
                ir::TypeExpr::NilableType(self.nilable_type(nilable_type)?)
            }
            ast::TypeExpr::LitType(lit_type) => ir::TypeExpr::LitType(self.lit_type(lit_type)?),
            ast::TypeExpr::AnyType(any_type) => ir::TypeExpr::AnyType(self.any_type(any_type)?),
        })
    }

    fn name_type(&self, item: ast::Name) -> Option<ir::NameType> {
        Some(ir::NameType {
            value: item.text()?,
        })
    }

    // fn item(&self, item: ast::Item) -> Option<ir_def::Item> {
    //     Some(match item {
    //         ast::Item::FnItem(fn_item) => ir_def::Item::FnItem(self.fn_item(fn_item)?),
    //     })
    // }
    //
    // fn fn_item(&self, item: ast::FnItem) -> Option<ir_def::FnItem> {
    //     Some(ir_def::FnItem {
    //         node_ptr: Some(item.node_ptr()),
    //         name: self.name(item.name()?)?,
    //         params: item
    //             .params()
    //             .map(|p| p.params().filter_map(|p| self.param(p)).collect())
    //             .unwrap_or_default(),
    //         output: item.output().and_then(|o| self.output(o)),
    //         body: item.body().and_then(|b| self.body(b)).unwrap_or_default(),
    //     })
    // }
    //
    //
    // fn body(&self, item: ast::BlockExpr) -> Option<ir_def::BlockExpr> {
    //     match self.expr(ast::Expr::BlockExpr(item))? {
    //         ir_def::Expr::BlockExpr(b) => Some(b),
    //         _ => None,
    //     }
    // }
    //
    // fn param(&self, item: ast::FnParam) -> Option<ir_def::FnParam> {
    //     Some(ir_def::FnParam {
    //         node_ptr: Some(item.node_ptr()),
    //         name: self.name(item.name()?)?,
    //         ty: self.type_expr(item.ty()?)?,
    //         //match is needed to exit if expr() fails (same with other optional fields like output)
    //         default_value: match item.default_value() {
    //             Some(e) => Some(self.expr(e)?),
    //             None => None,
    //         },
    //     })
    // }
    //
    //
    fn lit_type(&self, item: ast::LitType) -> Option<ir::LitType> {
        Some(ir::LitType { kind: item.kind()? })
    }

    fn any_type(&self, item: ast::AnyType) -> Option<ir::AnyType> {
        Some(ir::AnyType {})
    }

    fn nilable_type(&self, item: ast::NilableType) -> Option<ir::NilableType> {
        Some(ir::NilableType {
            value: self.type_expr(item.ty()?)?.into(),
        })
    }
    //
    // fn expr(&self, item: ast::Expr) -> Option<ir_def::Expr> {
    //     Some(match item {
    //         ast::Expr::LitExpr(lit_expr) => self.lit_expr(lit_expr).map_or_else(
    //             || ir_def::Expr::Missing(ir_def::Missing::default()),
    //             ir_def::Expr::LitExpr,
    //         ),
    //         ast::Expr::BinaryExpr(binary_expr) => self.binary_expr(binary_expr).map_or_else(
    //             || ir_def::Expr::Missing(ir_def::Missing::default()),
    //             ir_def::Expr::BinaryExpr,
    //         ),
    //         ast::Expr::UnaryExpr(unary_expr) => self.unary_expr(unary_expr).map_or_else(
    //             || ir_def::Expr::Missing(ir_def::Missing::default()),
    //             ir_def::Expr::UnaryExpr,
    //         ),
    //         ast::Expr::BlockExpr(block_expr) => self.block_expr(block_expr).map_or_else(
    //             || ir_def::Expr::Missing(ir_def::Missing::default()),
    //             ir_def::Expr::BlockExpr,
    //         ),
    //         ast::Expr::IndexExpr(index_expr) => self.index_expr(index_expr).map_or_else(
    //             || ir_def::Expr::Missing(ir_def::Missing::default()),
    //             ir_def::Expr::IndexExpr,
    //         ),
    //         ast::Expr::CallExpr(call_expr) => self.call_expr(call_expr).map_or_else(
    //             || ir_def::Expr::Missing(ir_def::Missing::default()),
    //             ir_def::Expr::CallExpr,
    //         ),
    //         ast::Expr::ParenExpr(paren_expr) => self.paren_expr(paren_expr).map_or_else(
    //             || ir_def::Expr::Missing(ir_def::Missing::default()),
    //             ir_def::Expr::ParenExpr,
    //         ),
    //         ast::Expr::NameExpr(name_expr) => self.name_expr(name_expr).map_or_else(
    //             || ir_def::Expr::Missing(ir_def::Missing::default()),
    //             ir_def::Expr::NameExpr,
    //         ),
    //         ast::Expr::ReturnExpr(return_expr) => self.return_expr(return_expr).map_or_else(
    //             || ir_def::Expr::Missing(ir_def::Missing::default()),
    //             ir_def::Expr::ReturnExpr,
    //         ),
    //     })
    // }
    //
    // fn expr_or_missing(&self, expr: Option<ast::Expr>) -> ir_def::Expr {
    //     expr.and_then(|e| self.expr(e))
    //         .unwrap_or_else(|| ir_def::Expr::Missing(ir_def::Missing::default()))
    // }
    //
    // fn lit_expr(&self, item: ast::LitExpr) -> Option<ir_def::LitExpr> {
    //     Some(ir_def::LitExpr {
    //         node_ptr: Some(item.node_ptr()),
    //         kind: item.kind()?,
    //     })
    // }
    //
    // fn binary_expr(&self, item: ast::BinaryExpr) -> Option<ir_def::BinaryExpr> {
    //     Some(ir_def::BinaryExpr {
    //         node_ptr: Some(item.node_ptr()),
    //         left: self.expr_or_missing(item.lhs()).into(),
    //         right: self.expr_or_missing(item.rhs()).into(),
    //         op: item.op_kind()?,
    //     })
    // }
    //
    // fn unary_expr(&self, item: ast::UnaryExpr) -> Option<ir_def::UnaryExpr> {
    //     Some(ir_def::UnaryExpr {
    //         node_ptr: Some(item.node_ptr()),
    //         expr: self.expr_or_missing(item.expr()).into(),
    //         kind: item.op_kind()?,
    //     })
    // }
    //
    // fn block_expr(&self, item: ast::BlockExpr) -> Option<ir_def::BlockExpr> {
    //     Some(ir_def::BlockExpr {
    //         node_ptr: Some(item.node_ptr()
    //         stmts: item.stmts().filter_map(|s| self.stmt(s)).collect_vec(),
    //     })
    // }
    //
    // fn stmt(&self, item: ast::Stmt) -> Option<ir_def::Stmt> {
    //     Some(match item {
    //         ast::Stmt::LetStmt(let_stmt) => ir_def::Stmt::LetStmt(self.let_stmt(let_stmt)?),
    //         ast::Stmt::ExprStmt(expr_stmt) => ir_def::Stmt::ExprStmt(self.expr_stmt(expr_stmt)?),
    //     })
    // }
    //
    // fn let_stmt(&self, item: ast::LetStmt) -> Option<ir_def::LetStmt> {
    //     Some(ir_def::LetStmt {
    //         node_ptr: Sme(item.node_ptr()),
    //         name: self.name(item.name()?)?,
    //         ty: self.type_expr(item.ty()?)?,
    //         expr: self.expr_or_missing(item.expr()),
    //     })
    // }
    //
    // fn expr_stmt(&self, item: ast::ExprStmt) -> Option<ir_def::ExprStmt> {
    //     Some(ir_def::ExprStmt {
    //         node_ptr: Some(item.node_ptr()),
    //         expr: self.expr_or_missing(item.expr()),
    //     })
    // }
    //
    // fn index_expr(&self, item: ast::IndexExpr) -> Option<ir_def::IndexExpr> {
    //     Some(ir_def::IndexExpr {
    //         node_ptr: Some(item.node_ptr()),
    //         base: self.expr_or_missing(item.base()).into(),
    //         index: self.expr_or_missing(item.index()).into(),
    //     })
    // }
    //
    // fn paren_expr(&self, item: ast::ParenExpr) -> Option<ir_def::ParenExpr> {
    //     Some(ir_def::ParenExpr {
    //         node_ptr: Some(item.node_ptr()),
    //         expr: self.expr_or_missing(item.expr()).into(),
    //     })
    // }
    //
    // fn call_expr(&self, item: ast::CallExpr) -> Option<ir_def::CallExpr> {
    //     dbg!(item.func());
    //     Some(ir_def::CallExpr {
    //         node_ptr: Some(item.node_ptr()),
    //         func: self.expr_or_missing(item.func()).into(),
    //         args: item
    //             .args()
    //             .map(|args| args.args().filter_map(|a| self.arg(a)).collect_vec())
    //             .unwrap_or_default(),
    //     })
    // }
    //
    // fn name_expr(&self, item: ast::NameExpr) -> Option<ir_def::NameExpr> {
    //     Some(ir_def::NameExpr {
    //         node_ptr: Some(item.node_ptr()),
    //         value: self.name(item.name()?)?,
    //     })
    // }
    //
    // fn return_expr(&self, item: ast::ReturnExpr) -> Option<ir_def::ReturnExpr> {
    //     Some(ir_def::ReturnExpr {
    //         node_ptr: Some(item.node_ptr()),
    //         expr: self.expr_or_missing(item.expr()).into(),
    //     })
    // }
    //
    // fn arg(&self, item: ast::Arg) -> Option<ir_def::Arg> {
    //     Some(ir_def::Arg {
    //         node_ptr: Some(item.node_ptr()),
    //         name: item.name().and_then(|n| self.name(n)),
    //         value: self.expr_or_missing(item.value()),
    //     })
    // }
    //
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    // use super::*;
    // use crate::{
    //     def::lower::{IrFile, lower_file},
    //     parsing::parser::{self, Parse},
    // };

    #[test]
    fn func() {
        // let parse: Arc<Parse> = parser::parse("fn test() {print(\"hello world!\");}").into();
        // assert!(parse.errors.is_empty());
        // insta::assert_debug_snapshot!(lower_file(parse).ir)
    }
}
