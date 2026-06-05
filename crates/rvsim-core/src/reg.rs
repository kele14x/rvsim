pub struct RegFile {
    x: [u32; 32],
}

impl Default for RegFile {
    fn default() -> Self {
        Self::new()
    }
}

impl RegFile {
    pub fn new() -> Self {
        Self { x: [0; 32] }
    }

    pub fn get(&self, reg: u8) -> u32 {
        self.x[reg as usize]
    }

    pub fn set(&mut self, reg: u8, val: u32) {
        if reg != 0 {
            self.x[reg as usize] = val;
        }
    }
}

pub struct FpRegFile {
    f: [u64; 32],
}

impl Default for FpRegFile {
    fn default() -> Self {
        Self::new()
    }
}

impl FpRegFile {
    pub fn new() -> Self {
        Self { f: [0; 32] }
    }

    pub fn get(&self, reg: u8) -> u64 {
        self.f[reg as usize]
    }

    pub fn set(&mut self, reg: u8, val: u64) {
        self.f[reg as usize] = val;
    }

    pub fn get_f32(&self, reg: u8) -> u32 {
        let raw = self.f[reg as usize];
        if raw & 0xFFFF_FFFF_0000_0000 == 0xFFFF_FFFF_0000_0000 {
            raw as u32
        } else {
            0x7FC0_0000
        }
    }

    pub fn set_f32(&mut self, reg: u8, val: u32) {
        self.f[reg as usize] = val as u64 | 0xFFFF_FFFF_0000_0000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x0_always_zero() {
        let mut rf = RegFile::new();
        rf.set(0, 0xDEADBEEF);
        assert_eq!(rf.get(0), 0);
    }

    #[test]
    fn read_write_general() {
        let mut rf = RegFile::new();
        rf.set(1, 42);
        assert_eq!(rf.get(1), 42);
        rf.set(31, 0xFFFF_FFFF);
        assert_eq!(rf.get(31), 0xFFFF_FFFF);
    }

    #[test]
    fn fp_f0_writable() {
        let mut fp = FpRegFile::new();
        fp.set(0, 0xDEAD_BEEF_CAFE_BABE);
        assert_eq!(fp.get(0), 0xDEAD_BEEF_CAFE_BABE);
    }

    #[test]
    fn fp_read_write_f64() {
        let mut fp = FpRegFile::new();
        fp.set(1, 0x3FF0_0000_0000_0000); // 1.0f64
        assert_eq!(fp.get(1), 0x3FF0_0000_0000_0000);
    }

    #[test]
    fn fp_nan_boxing() {
        let mut fp = FpRegFile::new();
        fp.set_f32(1, 0x3F80_0000); // 1.0f32
        assert_eq!(fp.get(1), 0xFFFF_FFFF_3F80_0000);
        assert_eq!(fp.get_f32(1), 0x3F80_0000);

        // Non-NaN-boxed value returns canonical NaN
        fp.set(2, 0x0000_0000_3F80_0000);
        assert_eq!(fp.get_f32(2), 0x7FC0_0000);
    }
}
