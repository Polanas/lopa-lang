// Buf.__index.put_uleb128_33 = function(self,  v, numbit)
//     local offs = self.offs
//     local b = bor(shl(band(v, 0x3f), 1), numbit)
//     v = shr(v, 6)
//     if v ~= 0 then b = bor(b, 0x80) end
//     self:put(b)
//     while v > 0 do
//         b = band(v, 0x7f)
//         v = shr(v, 7)
//         if v ~= 0 then b = bor(b, 0x80) end
//         self:put(b)
//     end
//     return offs
// end
pub trait WriteULEB128_33Ext: std::io::Write {
    fn write_uleb128_u32_33(&mut self, mut value: u32, flag: bool) -> std::io::Result<()> {
        {
            let mut byte = ((value & 0x3f) << 1) | (flag as u32);
            value >>= 6;
            if value != 0 {
                byte |= 0x80;
            }
            self.write_all(&[byte as u8])?;

            while value > 0 {
                let mut byte = value & 0x7f;
                value >>= 7;
                if value != 0 {
                    byte |= 0x80;
                }
                self.write_all(&[byte as u8])?;
            }

            Ok(())
        }
    }
}

// pub trait WriteULEB128_33Ext: std::io::Write {
//     fn write_uleb128_u32_33(&mut self, value: u32, flag: bool) -> std::io::Result<()> {
//         {
//             if value < 0x40 {
//                 dbg!(value);
//                 self.write_all(&[value as u8 | (flag as u8)])?;
//                 return Ok(());
//             }
//
//             let mut value = value;
//             let mut byte = value & 0x3F;
//             value >>= 6;
//             byte |= !0x3F;
//             self.write_all(&[byte as u8 | (flag as u8)])?;
//             if value == 0 {
//                 return Ok(());
//             }
//
//             loop {
//                 let mut byte = value & 0x7F;
//                 value >>= 7;
//                 if value != 0 {
//                     byte |= !0x7F;
//                 }
//                 self.write_all(&[byte as u8])?;
//                 if value == 0 {
//                     return Ok(());
//                 }
//             }
//         }
//     }
// }
impl<T: std::io::Write> WriteULEB128_33Ext for T {}
