#[allow(unused)]
mod shared_mut;

use std::{error::Error, io::Read};

use byteorder::ReadBytesExt;
use lopa_lang::{
    code_gen,
    luajit::{
        self, ABC, AD, BytecodeDump, ConstantTable, GCConstant, Instruction, NumberConstant,
        OpCode, Proto, ProtoFlags, TableValue,
    },
    parser, tokenizer,
    uleb128_33::WriteULEB128_33Ext,
};
use uleb128::{ReadULeb128Ext, WriteULeb128Ext};

fn test(x: i32) -> i32 {
    println!("x = {x}");
    x
}
macro_rules! I {
    ($opcode: ident,$a:expr, $d: expr) => {
        Instruction::AD(OpCode::$opcode, AD::new($a, $d))
    };
    ($opcode: ident,$a:expr, $b: expr, $c: expr) => {
        Instruction::ABC(OpCode::$opcode, ABC::new($a, $b, $c))
    };
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut b = std::io::Cursor::new(include_bytes!("../binary").to_vec());
    b.read_u8();
    b.read_u8();
    b.read_u8();
    let version = b.read_u8().unwrap();
    let flags = b.read_uleb128_u32().unwrap();
    let program = "";
    let tokens = tokenizer::tokenize(program);
    let ast = parser::parse_program(&tokens);
    match ast {
        Ok(ast) => {
            let proto = Proto {
                flags: ProtoFlags::empty(),
                num_pararms: 0,
                frame_size: 5,
                upvalues: vec![],
                gc_constants: vec![
                    // GCConstant::Str(String::from("print")),
                    // GCConstant::Table(ConstantTable {
                    //     array_part: vec![TableValue::Int(1)],
                    //     hash_part: vec![],
                    // }),
                ],
                number_constants: vec![NumberConstant::Num((i32::MAX as u32 + 1) as f64)],
                instructions: vec![
                    I!(KNUM, 0, 0),
                    I!(KSHORT, 1, 2),
                    // I!(ADDVV, 0, 0, 1),
                    // I!(GGET, 2, 0),
                    // I!(MOV, 4, 0),
                    // I!(CALL, 2, 1, 2),
                    I!(RET0, 0, 1),
                ],
            };
            let mut dump = BytecodeDump::new();
            dump.write_proto(proto);
            let data = dump.finish();
            println!("{:x?}", data);
            let lua = unsafe { mlua::Lua::unsafe_new() };
            lua.load(&data).exec().unwrap();
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
