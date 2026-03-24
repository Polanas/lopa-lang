#![allow(dead_code)]

use itertools::Itertools;
use logos::Logos;
use lopa_lang::lsp::{
    self, lexer::Syntax, parser::{Cst, Prettify}
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let text = "fn test(a: int, b: int) -> int {
            let x = vec[0](0)[0];
    }";
    dbg!(Syntax::lexer(text).collect_vec());
    let (node, errors) = lsp::parser::parse(text);
    println!("{}", Cst(node).prettify());
    for error in errors {
        println!("expected {:?}, got {}", &error.expected, &text[error.span]);
    }
    Ok(())
}
// // use lopa_lang::{parser, position, tokenizer, type_check};
// fn main() -> Result<(), Box<dyn Error>> {
//     dbg!(syn::parse_str::<TokenStream>("a >>= b").unwrap());
//
//     Ok(())
//     //TODO: unknown types as fn params are not checked
//
//     // let source = r#"
//     //     fn main(){
//     //         let x = x or y and z;
//     //     }
//     // "#;
//     // let tokens = tokenizer::tokenize(source);
//     // let ast = parser::parse_program(&tokens);
//     // match ast {
//     //     Ok(ast) => {
//     //         dbg!(ast);
//     //         // let mut type_context = type_check::TypeCheck::new();
//     //         // type_context.set_source(source);
//     //         // type_context.check(&ast);
//     //         // if !type_context.diagnostics().is_empty() {
//     //         //     let offsets = position::LineOffsets::new(source);
//     //         //     for error in type_context.diagnostics() {
//     //         //         println!(
//     //         //             "[ERROR on line {}]: {}",
//     //         //             offsets.line(error.span.start),
//     //         //             error.message
//     //         //         );
//     //         //     }
//     //         // } else {
//     //         //     // let code = code_gen::generate(&ast);
//     //         //     // println!("------------------------------");
//     //         //     // println!("{code}");
//     //         //     // let lua = mlua::Lua::new();
//     //         //     // lua.load(&code).exec().unwrap();
//     //         //     // lua.load("main()").exec().unwrap();
//     //         // }
//     //         // type_context.debug_dump();
//     //     }
//     //     Err(errs) => {
//     //         for error in errs {
//     //             println!("ERROR: {}", error.message);
//     //             println!(
//     //                 "{}",
//     //                 &source[(((error.span.start.0 as i32) - 2).min(0) as usize)
//     //                     ..((error.span.end.0) + 2).min(source.len())]
//     //             );
//     //         }
//     //     }
//     // }
//     // Ok(())
// }
