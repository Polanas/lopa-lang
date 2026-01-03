use std::collections::HashMap;

use itertools::{Itertools, Position};

use crate::{ast, common, position::WithSpan};

const STACK_IDENT: &str = "__stack_var__";
const SCOPE_IDENT: &str = "__scope__local__";

type ScopeId = usize;

#[derive(Debug, Default)]
struct Scope {
    stack: usize,
    locals: HashMap<String, String>,
    id: ScopeId,
}

impl Clone for Scope {
    fn clone(&self) -> Self {
        let mut scope = Self {
            stack: self.stack,
            id: self.id + 1,
            locals: Default::default(),
        };
        for local in self.locals.keys() {
            scope.insert_local(local.as_str());
        }
        scope
    }
}

impl Scope {
    fn new(id: ScopeId) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    fn push(&mut self) -> usize {
        let stack = self.stack;
        self.stack += 1;
        stack
    }

    fn stack_ident(&self, id: usize) -> String {
        format!("stack_{id}")
    }

    fn push_ident(&mut self) -> String {
        let push = self.push();
        self.stack_ident(push)
    }

    fn insert_local(&mut self, name: &str) -> &str {
        self.locals
            .insert(name.to_owned(), format!("{}{}", SCOPE_IDENT, name));
        self.locals[name].as_str()
    }

    fn ident<'a>(&'a self, name: &'a str) -> &'a str {
        self.locals.get(name).map(|n| n.as_str()).unwrap_or(name)
    }

    fn clear(&mut self) {
        self.stack = 0;
    }
}

pub struct Context {
    result: String,
    scopes: Vec<Scope>,
}

impl Context {
    fn new() -> Self {
        Self {
            result: Default::default(),
            scopes: vec![Scope::new(0)],
        }
    }

    fn scope(&self) -> &Scope {
        &self.scopes[self.scopes.len() - 1]
    }

    fn scope_mut(&mut self) -> &mut Scope {
        let len = self.scopes.len() - 1;
        &mut self.scopes[len]
    }

    fn push_scope(&mut self) {
        let scope = self.scope().clone();
        self.scopes.push(scope)
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn generate(&mut self, program: &[WithSpan<ast::Stmt>]) {
        match program {
            [item] => {
                let stmt = self.stmt(&item.value);
                self.scope_mut().clear();
                self.result.push_str(&stmt);
            }
            [items @ .., last] => {
                for item in items {
                    let stmt = self.stmt(&item.value);
                    self.scope_mut().clear();
                    self.result.push_str(&format!("{stmt}\n"));
                }
                let stmt = self.stmt(&last.value);
                self.result.push_str(&stmt);
            }
            [] => {}
        }
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> String {
        let mut result = String::new();
        match &stmt {
            ast::Stmt::Expr(stmt_expr) => {
                for expr in &stmt_expr.exprs {
                    if let Some(expr) = self.expr(&expr.value) {
                        result.push_str(&format!("{expr}\n"));
                    }
                }
            }
            ast::Stmt::Item(item) => todo!(),
            ast::Stmt::Assign(ast::Assign { idents, values, .. }) => {
                result.push_str(&self.binding(ast::BindingRef {
                    kind: common::BindingKind::Global,
                    idents: idents.as_slice(),
                    values: values.as_ref().map(|v| v.as_slice()),
                }));
            }
            ast::Stmt::Binding(binding) => {
                result.push_str(&self.binding(binding.as_ref()));
            }
            ast::Stmt::Print(e) => {
                if let Some(expr) = self.expr(&e.value) {
                    self.result.push_str(&format!("print({expr})"));
                }
            }
            ast::Stmt::Empty => todo!(),
        };
        result
    }

    fn binding(&mut self, binding: ast::BindingRef) -> String {
        let mut result = String::new();
        for ident in binding.idents {
            if binding.kind == common::BindingKind::Local {
                self.scope_mut().insert_local(&ident.value);
            }
        }
        if let Some(values) = binding.values {
            if binding.idents.len() == values.len()
                && values
                    .iter()
                    .all(|v| !(matches!(&v.value, ast::Expr::If(_) | ast::Expr::Block(_, _))))
            {
                if binding.kind == common::BindingKind::Local {
                    result.push_str("local ");
                }

                self.push_binding_idents(&binding, &mut result);
                result.push_str(" = ");
                for (pos, value) in values.iter().with_position() {
                    match pos {
                        Position::First | Position::Middle => {
                            if let Some(expr) = &self.expr(&value.value) {
                                result.push_str(&format!("{expr},"));
                            }
                        }
                        Position::Last | Position::Only => {
                            if let Some(expr) = &self.expr(&value.value) {
                                result.push_str(expr);
                            }
                        }
                    }
                }
            } else {
                for item in values {
                    match &item.value {
                        item @ (ast::Expr::Block(_, _) | ast::Expr::If(_)) => {
                            self.expr(item);
                        }
                        _ => {
                            if let Some(expr) = self.expr(&item.value) {
                                result.push_str(&format!(
                                    "{} = {}\n",
                                    self.scope_mut().push_ident(),
                                    expr
                                ));
                            }
                        }
                    }
                }
                let mut stack = self.scope().stack;
                for ident in binding.idents.iter().rev() {
                    stack -= 1;
                    if binding.kind == common::BindingKind::Local {
                        result.push_str("local ");
                    }
                    let stack_ident = self.scope().stack_ident(stack);
                    result.push_str(&format!(
                        "{} = {}\n",
                        self.scope_mut().ident(&ident.value),
                        stack_ident
                    ));
                }
            }
        } else {
            if binding.kind == common::BindingKind::Local {
                result.push_str("local ");
            }
            self.push_binding_idents(&binding, &mut result);
        }
        result
    }

    fn push_binding_idents(&mut self, binding: &ast::BindingRef<'_>, result: &mut String) {
        for (pos, ident) in binding.idents.iter().with_position() {
            match pos {
                Position::First | Position::Middle => {
                    result.push_str(&format!("{},", self.scope_mut().ident(&ident.value)));
                }
                Position::Last | Position::Only => {
                    result.push_str(self.scope_mut().ident(&ident.value));
                }
            }
        }
    }

    fn expr(&mut self, expr: &ast::Expr) -> Option<String> {
        match &expr {
            ast::Expr::Nil => Some(String::from("nil")),
            ast::Expr::Number(number) => match number {
                ast::Number::Float(f) => Some(f.to_string()),
                ast::Number::Int(i) => Some(i.to_string()),
            },
            ast::Expr::Bool(b) => Some(b.to_string()),
            ast::Expr::String(s) => Some(format!("\"{s}\"")),
            ast::Expr::Grouping(e) => self.expr(&e.value),
            ast::Expr::Unary(ast::UnaryExpr { expr, op, .. }) => match op.value {
                common::UnaryOp::Not => self.expr(&expr.value).map(|e| format!("not ({})", e)),
                common::UnaryOp::Negate => self.expr(&expr.value).map(|e| format!("-{}", e)),
            },
            ast::Expr::Binary(binary_expr) => {
                let op = match binary_expr.op.value {
                    common::BinaryOp::Div => "/",
                    common::BinaryOp::Mult => "*",
                    common::BinaryOp::Add => "+",
                    common::BinaryOp::Sub => "-",
                    common::BinaryOp::Greater => ">",
                    common::BinaryOp::GreaterEqual => ">=",
                    common::BinaryOp::Less => "<",
                    common::BinaryOp::LessEqual => "<=",
                    common::BinaryOp::NotEqual => "~=",
                    common::BinaryOp::Equal => "==",
                    common::BinaryOp::Modulo => "%",
                    common::BinaryOp::And => " and ",
                    common::BinaryOp::Or => " or ",
                };
                if let (Some(l), Some(r)) = (
                    self.expr(&binary_expr.left.value),
                    self.expr(&binary_expr.right.value),
                ) {
                    Some(format!("{}{}{}", l, op, r))
                } else {
                    None
                }
            }
            ast::Expr::Identifier(ident, _) => Some(self.scope_mut().ident(ident).to_owned()),
            ast::Expr::Call(_, items) => todo!(),
            ast::Expr::If(if_expr) => {
                let condition = self.expr(&if_expr.condition.value).unwrap();
                self.result.push_str(&format!("if {} then\n", &condition));
                let ident = self.block(&if_expr.then_branch);
                if let Some(else_branch) = &if_expr.else_branch {
                    self.pop_scope();
                    self.result.push_str("else\n");
                    self.expr(&else_branch.value);
                }
                self.result.push_str("end\n");
                ident
            }
            ast::Expr::Block(items, _) => self.block(items),
        }
    }

    fn block_last(&mut self, item: &ast::Stmt) -> Option<String> {
        if let ast::Stmt::Expr(expr) = &item
            && expr.semi.is_none()
        {
            match expr.exprs.as_slice() {
                [item] => {
                    let expr = self.expr(&item.value);
                    if let Some(expr) = expr {
                        let ident = self.scope_mut().push_ident();
                        self.result.push_str(&format!("{} = {}\n", ident, expr));
                        Some(ident)
                    } else {
                        None
                    }
                }
                [_item, ..] => {
                    for item in &expr.exprs {
                        let expr = self.expr(&item.value);
                        if let Some(expr) = expr {
                            let ident = self.scope_mut().push_ident();
                            self.result.push_str(&format!("{} = {}\n", ident, expr));
                        }
                    }
                    None
                }
                [] => unreachable!(),
            }
        } else {
            let stmt = self.stmt(item);
            self.result.push_str(&format!("{stmt}\n"));
            None
        }
    }

    fn block(&mut self, stmts: &[WithSpan<ast::Stmt>]) -> Option<String> {
        self.push_scope();
        match stmts {
            [item] => {
                self.result.push_str("do\n");
                let last = self.block_last(&item.value);
                self.result.push_str("end\n");
                last
            }
            [items @ .., last] => {
                self.result.push_str("do\n");
                for item in items {
                    let stmt = self.stmt(&item.value);
                    self.result.push_str(&format!("{stmt}\n"));
                }
                let result = self.block_last(&last.value);
                self.result.push_str("end\n");
                result
            }
            [] => None,
        }
    }
}

pub fn generate(program: &[WithSpan<ast::Stmt>]) -> String {
    let mut context = Context::new();
    context.generate(program);
    context.result
}

#[cfg(test)]
mod test {
    use crate::{code_gen, parser, position::Diagnostic, tokenizer};

    fn run(source: &str) -> Result<String, Vec<Diagnostic>> {
        let code = code_gen::generate(&parser::parse_program(&tokenizer::tokenize(source))?);
        let lua = mlua::Lua::new();
        lua.load(
            r#"
print = function(item)
    result = result and result .. ',' .. item or item
end"#,
        )
        .exec()
        .unwrap();

        lua.load(&code).exec().unwrap();
        Ok(lua.globals().get("result").unwrap())
    }

    #[test]
    fn blocks_multivalues() {
        assert_eq!(run("let x,y = 1,2; print x; print y").as_deref(), Ok("1,2"));
        assert_eq!(
            run("let x,y = 1,{2}; print x; print y").as_deref(),
            Ok("1,2")
        );
        assert_eq!(
            run("let x,y = 1,{2}-1; print x; print y").as_deref(),
            Ok("1,1")
        );
        assert_eq!(
            run("let x,y,z = 1, if true {2,3} else {4,5}; print x; print y; print z").as_deref(),
            Ok("1,2,3")
        );
        assert_eq!(
            run("let x = 2+{if true {1} else {2}}; print x;").as_deref(),
            Ok("3")
        );
        assert_eq!(run("let x = {0}+{1}; print x;").as_deref(), Ok("1"));
    }

    #[test]
    fn shadowing() {
        assert_eq!(
            run("
let y = 1;
let x = {
    let x = 20;
    let y = 2;
    x + y
};
print x;
print y;
 ")
            .as_deref(),
            Ok("22,1")
        );
    }
}
