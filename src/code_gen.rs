use std::collections::HashMap;

use itertools::{Itertools, Position};

trait Output {
    fn line(&mut self, text: &str);
    fn stmt(&mut self, text: &str);
}

impl Output for String {
    fn line(&mut self, text: &str) {
        self.push_str(text);
        self.push('\n');
    }

    fn stmt(&mut self, text: &str) {
        self.push_str(text);
        self.push_str(";\n");
    }
}

use crate::{
    ast::{self, Item},
    common,
    position::WithSpan,
    types,
};

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

#[derive(Debug)]
pub struct FnContext {
    returns: Vec<types::Type>,
    output: String,
}

pub struct Context {
    scopes: Vec<Scope>,
    call_stack: Vec<FnContext>,
    output: String,
}

impl Context {
    fn new() -> Self {
        Self {
            scopes: vec![Scope::new(0)],
            call_stack: vec![],
            output: Default::default(),
        }
    }

    fn scope(&self) -> &Scope {
        &self.scopes[self.scopes.len() - 1]
    }

    fn scope_mut(&mut self) -> &mut Scope {
        let len = self.scopes.len() - 1;
        &mut self.scopes[len]
    }

    fn call_stack(&self) -> &FnContext {
        &self.call_stack[self.call_stack.len() - 1]
    }

    fn call_stack_mut(&mut self) -> &mut FnContext {
        let len = self.call_stack.len() - 1;
        &mut self.call_stack[len]
    }

    fn push_call_stack(&mut self, context: FnContext) {
        self.call_stack.push(context);
    }

    fn pop_call_stack(&mut self) {
        self.call_stack.pop();
    }

    fn push_scope(&mut self) {
        let scope = self.scope().clone();
        self.scopes.push(scope)
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn generate(&mut self, program: &[WithSpan<Item>]) {
        for item in program {
            self.item(&item.value);
        }
    }

    fn item(&mut self, item: &Item) {
        match item {
            Item::Fn(func) => self.func(func),
            Item::Extern(_extern) => (),
            Item::Inline(inline) => inline
                .defs
                .iter()
                .for_each(|func| self.inline_func(&func.value)),
        }
    }

    fn inline_func(&mut self, func: &ast::InlineFn) {
        let args = func.params.iter().map(|p| &p.name.value).join(", ");
        self.output
            .line(&format!("{} = function({args})", &func.name));
        self.output.push_str(&func.body);
        self.output.stmt("end");
    }

    fn func(&mut self, func: &ast::Fn) {
        let args = func.params.iter().map(|p| &p.name.value).join(", ");
        self.output
            .line(&format!("{} = function({args})", &func.name));
        self.push_call_stack(FnContext {
            returns: func.returns.iter().map(|r| r.value.clone()).collect(),
            output: String::new(),
        });
        for param in func.params.iter() {
            if let Some(default_value) = &param.default_value {
                let default_value = self.expr(&default_value.value).unwrap();
                self.call_stack_mut().output.push_str(&format!(
                    r#"if {0} == nil then
  {0} = {1};
end;
"#,
                    &param.name.value, default_value
                ));
            }
        }
        self.block(&func.body.value);
        self.output
            .push_str(&self.call_stack.last().unwrap().output);
        if let Some(last) = func.body.value.body.last()
            && let ast::Stmt::Expr(ast::StmtExpr { semi: None, .. }) = &last.value
        {
            let stack = self.scope_mut().stack;
            let returns = ((stack - func.returns.len())..stack)
                .map(|i| self.scope_mut().stack_ident(i))
                .join(", ");
            self.output.stmt(&format!("return {}", returns));
        }
        self.pop_call_stack();
        self.output.stmt("end");
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> String {
        let mut result = String::new();
        match &stmt {
            ast::Stmt::Expr(stmt_expr) => {
                for expr in &stmt_expr.exprs {
                    if let Some(expr) = self.expr(&expr.value) {
                        result.stmt(&expr.to_string());
                    }
                }
            }
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
            ast::Stmt::Empty => {}
            ast::Stmt::Return(exprs) => {
                for expr in exprs {
                    match &expr.value {
                        item @ (ast::Expr::Block(_) | ast::Expr::If(_)) => {
                            self.expr(item);
                        }
                        _ => {
                            if let Some(expr) = self.expr(&expr.value) {
                                result.stmt(&format!(
                                    "{} = {}",
                                    self.scope_mut().push_ident(),
                                    expr
                                ));
                            }
                        }
                    }
                }
                let args = ((self.scope().stack - self.call_stack().returns.len())
                    ..self.scope().stack)
                    .map(|s| self.scope().stack_ident(s))
                    .join(", ");
                result.stmt(&format!("return {args}"));
            }
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
                    .all(|v| !(matches!(&v.value, ast::Expr::If(_) | ast::Expr::Block(_))))
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
                        item @ (ast::Expr::Block(_) | ast::Expr::If(_)) => {
                            self.expr(item);
                        }
                        _ => {
                            if let Some(expr) = self.expr(&item.value) {
                                result.stmt(&format!(
                                    "{} = {}",
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
                    result.stmt(&format!(
                        "{} = {}",
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
            ast::Expr::String(kind, s) => match kind {
                common::StringKind::Regular => Some(format!("\"{s}\"")),
                common::StringKind::Multiline => Some(format!("[[{s}]]")),
            },
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
            ast::Expr::Call(call) => self.call(call),
            ast::Expr::If(if_expr) => {
                let condition = self.expr(&if_expr.condition.value).unwrap();
                self.call_stack_mut()
                    .output
                    .line(&format!("if {} then", &condition));
                let ident = self.block(&if_expr.then_branch.value);
                if let Some(else_branch) = &if_expr.else_branch {
                    self.pop_scope();
                    self.call_stack_mut().output.line("else");
                    self.expr(&else_branch.value);
                }
                self.call_stack_mut().output.stmt("end");
                ident
            }
            ast::Expr::Block(block) => self.block(block),
            ast::Expr::Closure(closure) => self.closure(closure),
        }
    }

    fn closure(&mut self, closure: &ast::Closure) -> Option<String> {
        let args = closure.params.iter().map(|p| &p.name.value).join(", ");
        let mut result = String::new();
        result.line(&format!("function({args})"));
        self.push_call_stack(FnContext {
            returns: closure
                .returns
                .iter()
                .flatten()
                .map(|r| r.value.clone())
                .collect(),
            output: String::new(),
        });
        for param in closure.params.iter() {
            if let Some(default_value) = &param.default_value {
                let default_value = self.expr(&default_value.value).unwrap();
                self.call_stack_mut().output.push_str(&format!(
                    r#"if {0} == nil then
  {0} = {1};
end;
"#,
                    &param.name.value, default_value
                ));
            }
        }
        self.block(&closure.body.value);
        result.push_str(&self.call_stack.last().unwrap().output);
        if let Some(last) = closure.body.value.body.last()
            && let ast::Stmt::Expr(ast::StmtExpr { semi: None, .. }) = &last.value
        {
            let stack = self.scope_mut().stack;
            let returns = (stack - closure.returns.iter().flatten().count()..stack)
                .map(|i| self.scope_mut().stack_ident(i))
                .join(", ");
            result.stmt(&format!("return {}", returns));
        }
        self.pop_call_stack();
        result.line("end");
        Some(result)
    }

    fn call(&mut self, call: &ast::Call) -> Option<String> {
        let callee = self.expr(&call.callee.value)?;
        let types::TypeKind::Fn(func) = &call.callee_type.as_ref().unwrap().kind else {
            unreachable!();
        };
        let mut args: Vec<Option<String>> = vec![None; func.params.len()];

        let mut ordered_amount = 0;
        self.push_scope();
        //TODO: correctly handle multiple returns
        for arg in &call.args {
            let arg_expr = self.expr(&arg.expr.value);

            if let Some(name) = &arg.name {
                let (id, _) = func
                    .params
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.name.as_ref().map(|n| n == name).unwrap_or_default())
                    .unwrap();
                args[id] = Some(arg_expr.unwrap());
            } else {
                args[ordered_amount] = Some(arg_expr.unwrap());
                ordered_amount += 1;
            }
        }
        self.pop_scope();

        let args = args
            .into_iter()
            .map(|a| a.unwrap_or_else(|| "nil".to_string()))
            .join(", ");

        let call_string = format!("{callee}({args})");
        if func.returns.len() <= 1 {
            Some(call_string)
        } else {
            let idents = func
                .returns
                .iter()
                .map(|_| self.scope_mut().push_ident())
                .join(", ");
            self.call_stack_mut().output.stmt(&format!("{idents} = {call_string}"));
            None
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
                        self.call_stack_mut()
                            .output
                            .stmt(&format!("{} = {}", ident, expr));
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
                            self.call_stack_mut()
                                .output
                                .stmt(&format!("{} = {}", ident, expr));
                        }
                    }
                    None
                }
                [] => unreachable!(),
            }
        } else {
            let stmt = self.stmt(item);
            self.call_stack_mut().output.line(&stmt.to_string());
            None
        }
    }

    fn block(&mut self, block: &ast::Block) -> Option<String> {
        self.push_scope();
        match block.body.as_slice() {
            [item] => {
                self.call_stack_mut().output.line("do");
                let last = self.block_last(&item.value);
                self.call_stack_mut().output.stmt("end");
                last
            }
            [items @ .., last] => {
                self.call_stack_mut().output.line("do");
                for item in items {
                    let stmt = self.stmt(&item.value);
                    self.call_stack_mut().output.line(&stmt.to_string());
                }
                let result = self.block_last(&last.value);
                self.call_stack_mut().output.stmt("end");
                result
            }
            [] => None,
        }
    }
}

pub fn generate(program: &[WithSpan<Item>]) -> String {
    let mut context = Context::new();
    context.generate(program);
    context.output
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
