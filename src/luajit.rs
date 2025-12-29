use byteorder::{LittleEndian, WriteBytesExt};
use uleb128::WriteULeb128Ext;

use crate::uleb128_33::WriteULEB128_33Ext;

#[macro_export]
macro_rules! instruction {
    ($opcode: ident,$a:expr, $d: expr $(,)?) => {
        $crate::luajit::Instruction::AD(
            $crate::luajit::OpCode::$opcode,
            $crate::luajit::AD::new($a, $d),
        )
    };
    ($opcode: ident,$a:expr, $b: expr, $c: expr $(,)?) => {
        $crate::luajit::Instruction::ABC(
            $crate::luajit::OpCode::$opcode,
            $crate::luajit::ABC::new($a, $b, $c),
        )
    };
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum OpCode {
    ISLT,   // if A<VAR> < D<VAR> then JMP
    ISGE,   // if not (A<VAR> < D<VAR>) then JMP
    ISLE,   // if A<VAR> <= D<VAR> then JMP
    ISGT,   // if not (A<VAR> <= D<VAR>) then JMP
    ISEQV,  // if A<VAR> == D<VAR> then JMP
    ISNEV,  // if A<VAR> ~= D<VAR> then JMP
    ISEQS,  // if A<VAR> == D<STR> then JMP
    ISNES,  // if A<VAR> ~= D<STR> then JMP
    ISEQN,  // if A<VAR> == D<NUM> then JMP
    ISNEN,  // if A<VAR> ~= D<NUM> then JMP
    ISEQP,  // if A<VAR> == D<PRI> then JMP
    ISNEP,  // if A<VAR> ~= D<PRI> then JMP
    ISTC,   // if D<VAR> then A<DST> = D and JMP
    ISFC,   // if not D<VAR> then A<DST> = D and JMP
    IST,    // if D<VAR> then JMP
    ISF,    // if not D<VAR> then JMP
    ISTYPE, // unsupported
    ISNUM,  // unsupported
    MOV,    // A<DST> = D<VAR>
    NOT,    // A<DST> = not D<VAR>
    UNM,    // A<DST> = -D<VAR>
    LEN,    // A<DST> = #D<VAR>
    ADDVN,  // A<DST> = B<VAR> + C<NUM>
    SUBVN,  // A<DST> = B<VAR> - C<NUM>
    MULVN,  // A<DST> = B<VAR> * C<NUM>
    DIVVN,  // A<DST> = B<VAR> / C<NUM>
    MODVN,  // A<DST> = B<VAR> % C<NUM>
    ADDNV,  // A<DST> = C<NUM> + B<VAR>
    SUBNV,  // A<DST> = C<NUM> - B<VAR>
    MULNV,  // A<DST> = C<NUM> * B<VAR>
    DIVNV,  // A<DST> = C<NUM> / B<VAR>
    MODNV,  // A<DST> = C<NUM> % B<VAR>
    ADDVV,  // A<DST> = B<VAR> + C<VAR>
    SUBVV,  // A<DST> = B<VAR> - C<VAR>
    MULVV,  // A<DST> = B<VAR> * C<VAR>
    DIVVV,  // A<DST> = B<VAR> / C<VAR>
    MODVV,  // A<DST> = B<VAR> % C<VAR>
    POW,    // A<DST> = B<VAR> ^ C<VAR>
    CAT,    // A<DST> = B<RBASE> .. B++ -> C<RBASE>
    KSTR,   // A<DST> = D<STR>
    KCDATA, // A<DST> = D<CDATA>
    KSHORT, // A<DST> = D<LITS>
    KNUM,   // A<DST> = D<NUM>
    KPRI,   // A<DST> = D<PRI>
    KNIL,   // A<BASE>, A++ -> D<BASE> = nil
    UGET,   // A<DST> = D<UV>
    USETV,  // A<UV> = D<VAR>
    USETS,  // A<UV> = D<STR>
    USETN,  // A<UV> = D<NUM>
    USETP,  // A<UV> = D<PRI>
    UCLO,   // upvalue close for A<RBASE>, A++ -> framesize; goto D<JUMP>
    FNEW,   // A<DST> = D<FUNC>
    TNEW,   // A<DST> = {}
    TDUP,   // A<DST> = D<TAB>
    GGET,   // A<DST> = _G.D<STR>
    GSET,   // _G.D<STR> = A<VAR>
    TGETV,  // A<DST> = B<VAR>[C<VAR>]
    TGETS,  // A<DST> = B<VAR>[C<STR>]
    TGETB,  // A<DST> = B<VAR>[C<LIT>]
    TGETR,  // unsupported
    TSETV,  // B<VAR>[C<VAR>] = A<VAR>
    TSETS,  // B<VAR>[C<STR>] = A<VAR>
    TSETB,  // B<VAR>[C<LIT>] = A<VAR>
    TSETM,  // A-1<BASE>[D&0xFFFFFFFF<NUM>] <- A (<- multres)
    TSETR,  // unsupported
    CALLM, // if B<LIT> == 0 then A<BASE> (<- multres) <- A(A+FR2?2:1, A++ -> for C<LIT>, A++ (<- multres)) else A, A++ -> for B-1 = A(A+FR2?2:1, A++ -> for C, A++ (<- multres))
    CALL, // if B<LIT> == 0 then A<BASE> (<- multres) <- A(A+FR2?2:1, A++ -> for C-1<LIT>) else A, A++ -> for B-1 = A(A+FR2?2:1, A++ -> for C-1)
    CALLMT, // return A<BASE>(A+FR2?2:1, A++ -> for D<LIT>, A++ (<- multres))
    CALLT, // return A<BASE>(A+FR2?2:1, A++ -> for D-1<LIT>)
    ITERC, // for A<BASE>, A++ -> for B-1<LIT> in A-3, A-2, A-1 do
    ITERN, // for A<BASE>, A++ -> for B-1<LIT> in A-3, A-2, A-1 do
    VARG, // if B<LIT> == 0 then A<BASE> (<- multres) <- ... else A, A++ -> for B-1 = ...
    ISNEXT, // goto ITERN at D<JUMP>
    RETM, // return A<BASE>, A++ -> for D<LIT>, A++ (<- multres)
    RET,  // return A<RBASE>, A++ -> for D-1<LIT>
    RET0, // return
    RET1, // return A<RBASE>
    FORI, // for A+3<BASE> = A, A+1, A+2 do; exit at D<JUMP>
    JFORI, // unsupported
    FORL, // end of numeric for loop; start at D<JUMP>
    IFORL, // unsupported
    JFORL, // unsupported
    ITERL, // end of generic for loop; start at D<JUMP>
    IITERL, // unsupported
    JITERL, // unsupported
    LOOP, // if D<JUMP> == 32767 then goto loop else while/repeat loop; exit at D
    ILOOP, // unsupported
    JLOOP, // unsupported
    JMP,  // goto D<JUMP> or if true then JMP or goto ITERC at D
    FUNCF, // unsupported
    IFUNCF, // unsupported
    JFUNCF, // unsupported
    FUNCV, // unsupported
    IFUNCV, // unsupported
    JFUNCV, // unsupported
    FUNCC, // unsupported
    FUNCCW, // unsupported

    MAX = u8::MAX,
}

#[derive(Clone, Copy, Debug)]
pub struct ABC {
    a: u8,
    b: u8,
    c: u8,
}

impl ABC {
    pub fn new(a: u8, b: u8, c: u8) -> Self {
        Self { a, b, c }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AD {
    a: u8,
    d: u16,
}

impl AD {
    pub fn new(a: u8, d: u16) -> Self {
        Self { a, d }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    ABC(OpCode, ABC),
    AD(OpCode, AD),
}

pub const MAGIC: &[u8] = &[0x1B, 0x4C, 0x4A];
pub const VERSION: u8 = 2;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct HeaderFlags: u32 {
        ///If present, byte-code is interpreted as big-endian.
        const BIG_ENDIAN = 0x01;
        ///If present, debug-info is assumed to be stripped.
        const STRIP = 0x02;
        ///If present, FFI is enabled.
        const FFI = 0x04;
        const FR2 = 0x08;
        const KNOWN = 15;

        const _ = !0;
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct ProtoFlags: u8 {
        /// Has child prototypes.
        const HAS_CHILD = 0x01;
        /// Vararg function.
        const VARIADIC = 0x02;
        /// Uses BC_KCDATA for FFI datatypes.
        const FFI = 0x04;
        /// JIT disabled for this function.
        const NOJIT = 0x08;
        /// Patched bytecode with ILOOP etc.
        const ILOOP = 0x10;

        const _ = !0;
    }
}

#[derive(Clone, Debug, Copy)]
#[repr(u32)]
pub enum GCConstantKind {
    CHILD,
    TAB,
    I64,
    U64,
    Complex,
    Str,
}

/// Type codes for the GC constants of a prototype.
#[derive(Clone, Debug)]
pub enum GCConstant {
    Child,
    Table(ConstantTable),
    I64(i64),
    U64(u64),
    Complex(num_complex::Complex64),
    Str(String),
}

impl GCConstant {
    pub fn size(&self) -> u32 {
        1 + match self {
            GCConstant::Child => 0,
            GCConstant::Table(t) => t.size(),
            GCConstant::I64(i) => kgc_num_size(u64::from_ne_bytes(i.to_ne_bytes())),
            GCConstant::U64(u) => kgc_num_size(*u),
            GCConstant::Complex(c) => kgc_complex_size(*c),
            GCConstant::Str(s) => kgc_str_size(s),
        }
    }

    pub fn kind(&self) -> GCConstantKind {
        match self {
            GCConstant::Child => GCConstantKind::CHILD,
            GCConstant::Table(_) => GCConstantKind::TAB,
            GCConstant::I64(_) => GCConstantKind::I64,
            GCConstant::U64(_) => GCConstantKind::U64,
            GCConstant::Complex(_) => GCConstantKind::Complex,
            GCConstant::Str(_) => GCConstantKind::Str,
        }
    }

    pub fn write(&self, data: &mut Vec<u8>) {
        match self {
            GCConstant::Child => {
                data.push(GCConstantKind::CHILD as u8);
            }
            GCConstant::Table(constant_table) => constant_table.write(data),
            GCConstant::I64(i) => {
                data.push(GCConstantKind::I64 as u8);
                write_kgc_num(u64::from_ne_bytes(i.to_ne_bytes()), data);
            }
            GCConstant::U64(u) => {
                data.push(GCConstantKind::U64 as u8);
                write_kgc_num(*u, data);
            }
            GCConstant::Complex(complex) => {
                data.push(GCConstantKind::Complex as u8);
                write_kgc_complex(*complex, data);
            }
            GCConstant::Str(s) => {
                data.write_uleb128_u32(GCConstantKind::Str as u32 + s.len() as u32)
                    .unwrap();
                write_kgc_str(s, data);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConstantTable {
    pub array_part: Vec<TableValue>,
    pub hash_part: Vec<(TableValue, TableValue)>,
}

impl ConstantTable {
    pub fn size(&self) -> u32 {
        let mut size = uleb32_size(self.array_part.len() as _);
        size += uleb32_size(self.hash_part.len() as _);
        size += self.array_part.iter().fold(0, |size, v| size + v.size());
        size += self
            .hash_part
            .iter()
            .fold(0, |size, v| size + v.0.size() + v.1.size());
        size
    }
    pub fn write(&self, data: &mut Vec<u8>) {
        data.push(GCConstantKind::TAB as u8);
        data.write_uleb128_u32(self.array_part.len() as u32)
            .unwrap();
        data.write_uleb128_u32(self.hash_part.len() as u32).unwrap();
        for item in &self.array_part {
            item.write(data);
        }
        for (key, value) in &self.hash_part {
            key.write(data);
            value.write(data);
        }
    }
}

#[derive(Clone, Debug, Copy)]
#[repr(u32)]
pub enum TableValueKind {
    Nil,
    False,
    True,
    Int,
    Num,
    Str,
}

/// Type codes for the keys/values of a constant table.
#[derive(Clone, Debug)]
pub enum TableValue {
    Nil,
    False,
    True,
    Int(u32),
    Num(f64),
    Str(String),
}

impl TableValue {
    pub fn kind(&self) -> TableValueKind {
        match self {
            TableValue::Nil => TableValueKind::Nil,
            TableValue::False => TableValueKind::False,
            TableValue::True => TableValueKind::True,
            TableValue::Int(_) => TableValueKind::Int,
            TableValue::Num(_) => TableValueKind::Num,
            TableValue::Str(_) => TableValueKind::Str,
        }
    }

    pub fn size(&self) -> u32 {
        1 + match self {
            TableValue::Nil => 0,
            TableValue::False => 0,
            TableValue::True => 0,
            TableValue::Int(i) => kgc_int_size(*i),
            TableValue::Num(n) => kgc_num_size(n.to_bits()),
            TableValue::Str(s) => kgc_str_size(s),
        }
    }
    pub fn write(&self, data: &mut Vec<u8>) {
        match self {
            TableValue::Nil => {
                data.push(TableValueKind::Nil as u8);
            }
            TableValue::False => {
                data.push(TableValueKind::False as u8);
            }
            TableValue::True => {
                data.push(TableValueKind::True as u8);
            }
            TableValue::Int(i) => {
                data.push(TableValueKind::Int as u8);
                data.write_uleb128_u32(*i).unwrap();
            }
            TableValue::Num(n) => {
                data.push(TableValueKind::Num as u8);
                write_kgc_num(n.to_bits(), data);
            }
            TableValue::Str(s) => {
                data.push(TableValueKind::Str as u8);
                write_kgc_str(s, data);
            }
        }
    }
}

fn write_kgc_num(value: u64, data: &mut Vec<u8>) {
    data.write_uleb128_u32((value & 0xFFFFFFFF) as u32).unwrap();
    data.write_uleb128_u32((value >> 32) as u32).unwrap();
}

fn write_kgc_complex(value: num_complex::Complex64, data: &mut Vec<u8>) {
    write_kgc_num(value.re.to_bits(), data);
    write_kgc_num(value.im.to_bits(), data);
}

fn write_kgc_str(value: &str, data: &mut Vec<u8>) {
    data.extend_from_slice(value.as_bytes());
}

fn kgc_int_size(value: u32) -> u32 {
    uleb32_size(value)
}

fn kgc_int_33_size(value: u32) -> u32 {
    uleb32_33_size(value)
}

fn kgc_complex_size(value: num_complex::Complex64) -> u32 {
    kgc_num_size(value.re.to_bits()) + kgc_num_size(value.im.to_bits())
}

fn kgc_num_size(value: u64) -> u32 {
    uleb32_33_size((value >> 32) as u32) + uleb32_size((value & 0xFFFFFFFF) as u32)
}

fn kgc_str_size(value: &str) -> u32 {
    value.len() as u32
}

#[derive(Clone, Copy, Debug)]
pub enum NumberConstantKind {
    Int,
    Num,
}

#[derive(Clone, Copy, Debug)]
pub enum NumberConstant {
    Int(u32),
    Num(f64),
}

impl NumberConstant {
    pub fn size(self) -> u32 {
        match self {
            NumberConstant::Int(i) => kgc_int_33_size(i),
            NumberConstant::Num(n) => kgc_num_size(n.to_bits()),
        }
    }
    pub fn write(self, data: &mut Vec<u8>) {
        match self {
            NumberConstant::Int(i) => {
                data.write_uleb128_u32_33(i, false).unwrap();
            }
            NumberConstant::Num(n) => {
                let bits = n.to_bits();
                data.write_uleb128_u32_33((bits & 0xFFFFFFFF) as u32, true)
                    .unwrap();
                data.write_uleb128_u32((bits >> 32) as u32).unwrap();
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Upvalue {
    Local(u8),
    Closed(u8),
    ClosedReadonly(u8),
}

#[derive(Clone, Debug, Default)]
pub struct Proto {
    pub flags: ProtoFlags,
    /// Amount of function parameters.
    pub num_pararms: u8,
    /// Amount of slots required by the function.
    pub frame_size: u8,
    pub upvalues: Vec<Upvalue>,
    pub gc_constants: Vec<GCConstant>,
    pub number_constants: Vec<NumberConstant>,
    pub instructions: Vec<Instruction>,
    pub children: Vec<Proto>,
}

impl Proto {
    pub fn size(&self) -> u32 {
        let mut size = 4;

        size += uleb32_size(self.gc_constants.len() as _);
        size += uleb32_size(self.number_constants.len() as _);
        size += uleb32_size(self.instructions.len() as _);

        size += (self.instructions.len()) as u32 * 4;
        size += (self.upvalues.len() * 2) as u32;
        size += self.gc_constants.iter().fold(0, |size, c| size + c.size());
        size + self
            .number_constants
            .iter()
            .fold(0, |size, c| size + c.size())
    }

    pub fn write(&self, data: &mut Vec<u8>) {
        for child in &self.children {
            child.write(data);
        }
        self.write_header(data);
        self.write_bytecode(data);
        self.write_upvalues(data);
        self.write_gc_constants(data);
        self.write_number_constants(data);
    }

    pub fn write_header(&self, data: &mut Vec<u8>) {
        data.write_uleb128_u32(self.size()).unwrap();
        data.push(self.flags.bits());
        data.push(self.num_pararms as _);
        data.push(self.frame_size as _);
        data.push(self.upvalues.len() as _);
        data.write_uleb128_u32(self.gc_constants.len() as _)
            .unwrap();
        data.write_uleb128_u32(self.number_constants.len() as _)
            .unwrap();
        data.write_uleb128_u32(self.instructions.len() as _)
            .unwrap();
    }

    pub fn write_upvalues(&self, data: &mut Vec<u8>) {}

    pub fn write_gc_constants(&self, data: &mut Vec<u8>) {
        for constant in &self.gc_constants {
            constant.write(data);
        }
    }

    pub fn write_number_constants(&self, data: &mut Vec<u8>) {
        for constant in &self.number_constants {
            constant.write(data);
        }
    }

    pub fn write_bytecode(&self, data: &mut Vec<u8>) {
        for instruction in &self.instructions {
            match instruction {
                Instruction::ABC(op_code, abc) => {
                    data.write_u32::<LittleEndian>(
                        (*op_code as u32)
                            | ((abc.a as u32) << 8)
                            | ((abc.c as u32) << 16)
                            | ((abc.b as u32) << 24),
                    )
                    .unwrap();
                    // data.write_u32::<LittleEndian>(
                    //     ((*op_code as u32) << 24)
                    //         | ((abc.a as u32) << 16)
                    //         | ((abc.b as u32) << 8)
                    //         | (abc.c as u32),
                    // )
                    // .unwrap();
                }
                Instruction::AD(op_code, ad) => {
                    data.write_u32::<LittleEndian>(
                        (*op_code as u32) | ((ad.a as u32) << 8) | ((ad.d as u32) << 16),
                    )
                    .unwrap();
                }
            }
        }
    }
}

pub fn uleb32_size(value: u32) -> u32 {
    match value {
        _ if value <= 127 => 1,
        _ if value <= 16383 => 2,
        _ if value <= 2097151 => 3,
        _ if value <= 268435455 => 4,
        _ => 5,
    }
}

pub fn uleb32_33_size(value: u32) -> u32 {
    match value {
        _ if value <= 63 => 1,
        _ if value <= 8191 => 2,
        _ if value <= 1048575 => 3,
        _ if value <= 134217727 => 4,
        _ => 5,
    }
}

pub struct Context {
    data: Vec<u8>,
}

impl Context {
    pub fn new() -> Self {
        let mut dump = Self {
            data: Vec::with_capacity(1024),
        };
        dump.write_header();
        dump
    }

    pub fn write_header(&mut self) {
        self.data.extend_from_slice(MAGIC);
        self.data.push(VERSION);
        self.data
            .write_uleb128_u32((HeaderFlags::STRIP | HeaderFlags::FR2 | HeaderFlags::FFI).bits())
            .unwrap();
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.data.push(0);
        self.data
    }

    pub fn write_proto(&mut self, mut proto: Proto) {
        if !proto.children.is_empty() {
            proto.flags |= ProtoFlags::HAS_CHILD;
        }
        proto.write(&mut self.data);
    }

    // fn write_proto_inner(&mut self, proto: Proto, data: &mut [u8]) {
    //
    // }
}

#[cfg(test)]
mod test {
    use crate::luajit::{
        ABC, AD, Context, GCConstant, Instruction, NumberConstant, OpCode, Proto, ProtoFlags,
    };
    use instruction as I;

    #[test]
    fn proto_size() {
        let proto = Proto {
            flags: ProtoFlags::VARIADIC,
            num_pararms: 0,
            frame_size: 5,
            upvalues: vec![],
            gc_constants: vec![GCConstant::Str(String::from("print"))],
            number_constants: vec![NumberConstant::Num(3.5)],
            instructions: vec![
                I!(KNUM, 0, 0),
                I!(KSHORT, 1, 2),
                I!(GGET, 2, 0),
                I!(ADDVV, 4, 0, 1),
                I!(CALL, 2, 1, 2),
                I!(RET0, 0, 1),
            ],
            children: vec![],
        };

        assert_eq!(proto.size(), 43)
    }

    #[test]
    fn proto_write() {
        let proto = Proto {
            flags: ProtoFlags::VARIADIC,
            num_pararms: 0,
            frame_size: 5,
            upvalues: vec![],
            gc_constants: vec![GCConstant::Str(String::from("print"))],
            number_constants: vec![NumberConstant::Num(3.5)],
            instructions: vec![
                I!(KNUM, 0, 0),
                I!(KSHORT, 1, 2),
                I!(GGET, 2, 0),
                I!(MOV, 4, 0),
                I!(CALL, 2, 1, 2),
                I!(RET0, 0, 1),
            ],
            children: vec![],
        };
        let mut dump = Context::new();
        dump.write_proto(proto);
        let data = dump.finish();
        dbg!(data.len());
        println!("{:x?}", data);
    }
}
