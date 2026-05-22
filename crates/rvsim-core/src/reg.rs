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
}
