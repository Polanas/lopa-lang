#[allow(unused)]
mod shared_mut;

use std::error::Error;

use lopa_lang::{code_gen, parser, tokenizer};

fn main() -> Result<(), Box<dyn Error>> {
    let program = "
let x = (20+1)/4;
let y = x - 1;
let str = \"hi\";

print y;
print str;
print 2 + 2;";
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
