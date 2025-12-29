#[allow(unused)]
mod shared_mut;

use std::{error::Error, io::Read};

use lopa_lang::{code_gen, instruction, ir, luajit, parser, tokenizer};

fn main() -> Result<(), Box<dyn Error>> {
    // let mut context = luajit::Context::new();
    // let proto = luajit::Proto {
    //     gc_constants: vec![luajit::GCConstant::Str(String::from("print"))],
    //     instructions: vec![
    //         instruction!(GGET, 0, 0),
    //         instruction!(KSHORT, 2, 1),
    //         instruction!(UNM, 2, 2),
    //         instruction!(CALL, 0, 1, 2),
    //         instruction!(RET0, 0, 1),
    //     ],
    //     ..Default::default()
    // };
    // context.write_proto(proto);
    // let dump = context.finish();
    // mlua::Lua::new().load(&dump).exec().unwrap();
    let program = "x,y,z = 1,2,{1+2}; x,y,z=z,{y*2},x; print x; print y; print z";

    let tokens = tokenizer::tokenize(program);
    let ast = parser::parse_program(&tokens);
    match ast {
        Ok(ast) => {
            let ir_context = ir::generate(&ast);
            dbg!(&ir_context.instructions);
            let bytecode = code_gen::generate(ir_context);
            std::fs::write("binary", &bytecode).unwrap();
            mlua::Lua::new().load(&bytecode).exec().unwrap();
        }
        Err(errs) => {
            for error in errs {
                dbg!(
                    error.message,
                    Some(
                        &program[(((error.span.start.0 as i32) - 2).min(0) as usize)
                            ..(error.span.end.0) + 2]
                    )
                );
            }
        }
    }

    Ok(())
}
