#[allow(unused)]
mod shared_mut;

use std::error::Error;

use lopa_lang::{code_gen, parser, tokenizer};

fn test(x: i32) -> i32 {
    println!("x = {x}");
    x
}

fn main() -> Result<(), Box<dyn Error>> {
    let program = "let x,y,s = 1,2,3; print x; print y; print s;";
    println!("program: {program}");
    println!();
    let tokens = tokenizer::tokenize(program);
    let ast = parser::parse_program(&tokens);
    match ast {
        Ok(ast) => {
            let lua_code = code_gen::generate(&ast);
            println!("lua_code:\n{lua_code}\n");
            let lua = mlua::Lua::new();
            lua.load(lua_code).exec().unwrap();
        }
        Err(errs) => {
            for error in errs {
                dbg!(
                    error.message,
                    Some(&program[(error.span.start.0 - 2)..(error.span.end.0) + 2])
                );
            }
        }
    }

    Ok(())
}
