pub trait WriteULEB128_33Ext: std::io::Write {
    fn write_uleb128_u32_33(&mut self, value: u32, flag: bool) -> std::io::Result<()> {
        {
            if value < 0x40 {
                self.write_all(&[value as u8 | (flag as u8)])?;
                return Ok(());
            }

            let mut value = value;
            let mut byte = value & 0x3F;
            value >>= 6;
            byte |= !0x3F;
            byte <<= 1;
            self.write_all(&[byte as u8 | (flag as u8)])?;
            if value == 0 {
                return Ok(());
            }

            loop {
                let mut byte = value & 0x7F;
                value >>= 7;
                if value != 0 {
                    byte |= !0x7F;
                }
                self.write_all(&[byte as u8])?;
                if value == 0 {
                    return Ok(());
                }
            }
        }
    }
}

impl<T: std::io::Write> WriteULEB128_33Ext for T {}
