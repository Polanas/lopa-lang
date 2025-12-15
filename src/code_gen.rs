use crate::{ast, position};

pub fn generate(program: &[position::WithSpan<ast::Stmt>]) -> String {
    let mut stmt_string = String::new();
    let mut output = String::new();

    for stmt in program {
        generate_statement(stmt, &mut stmt_string);
        output.push_str(stmt_string.as_str());
        output.push('\n');
        stmt_string.clear();
    }

    output.remove(output.len() - 1);

    output
}

fn generate_statement(stmt: &position::WithSpan<ast::Stmt>, output: &mut String) {
    match &stmt.value {
        ast::Stmt::Expr(_) => {}
        ast::Stmt::Item(item) => {}
        ast::Stmt::Binding(binding) => {
            output.push_str(match binding.binding_type {
                ast::BindingType::Let => "local ",
                ast::BindingType::Global => "",
            });
            output.push_str(&binding.identifiers[0].value);
            if let Some(values) = &binding.values {
                output.push_str(" = ");

                generate_expr(&values[0], output);
            }
        }
        ast::Stmt::Print(expr) => {
            output.push_str("print(");
            generate_expr(expr, output);
            output.push(')');
        }
    }
}

fn generate_expr(expr: &position::WithSpan<ast::Expr>, output: &mut String) {
    match &expr.value {
        ast::Expr::Nil => output.push_str("nil"),
        ast::Expr::Number(number) => match number {
            ast::Number::Float(f) => output.push_str(&f.to_string()),
            ast::Number::Int(i) => output.push_str(&i.to_string()),
        },
        ast::Expr::Boolean(b) => output.push_str(&b.to_string()),
        ast::Expr::String(str) => {
            output.push('"');
            output.push_str(str);
            output.push('"');
        }
        ast::Expr::Grouping(e) => {
            output.push('(');
            generate_expr(e, output);
            output.push(')');
        }
        ast::Expr::Unary(op, expr) => {
            match op.value {
                ast::UnaryOp::Minus => output.push('-'),
                ast::UnaryOp::Not => output.push_str("not "),
            };
            generate_expr(expr, output);
        }
        ast::Expr::Binary(binary_expr) => {
            generate_expr(&binary_expr.left, output);
            output.push_str(binary_expr.op.value.to_lua());
            generate_expr(&binary_expr.right, output);
        }
        ast::Expr::Identifier(identifier) => output.push_str(&identifier),
        ast::Expr::Logical(logical_expr) => todo!(),
        ast::Expr::Assign(items, items1) => todo!(),
        ast::Expr::Call(_, items) => todo!(),
        ast::Expr::If(if_expr) => todo!(),
        ast::Expr::Block(items) => todo!(),
        ast::Expr::List(items) => todo!(),
    }
}
