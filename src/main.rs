#[allow(unused)]
mod shared_mut;

use std::error::Error;

use lopa_lang::{code_gen, parser, tokenizer};

fn test(x: i32) -> i32 {
    println!("x = {x}");
    x
}

fn main() -> Result<(), Box<dyn Error>> {
        let program = "
    let y =\"2\";
    let z=15;
    let w= 11;
    let x = {print \"1\"; 2} + { print y; 2 }*2+z+w;
    print x;";
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
            dbg!(errs);
        }
    }

    Ok(())
}
