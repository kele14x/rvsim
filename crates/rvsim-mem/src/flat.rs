use rvsim_core::mem::Memory;
use rvsim_core::trap::Trap;

pub struct FlatMemory {
    data: Vec<u8>,
    base: u32,
}

impl FlatMemory {
    pub fn new(size: usize, base: u32) -> Self {
        Self {
            data: vec![0u8; size],
            base,
        }
    }

    pub fn load(&mut self, offset: u32, data: &[u8]) {
        let start = offset.wrapping_sub(self.base) as usize;
        self.data[start..start + data.len()].copy_from_slice(data);
    }

    fn offset(&self, addr: u32) -> Option<usize> {
        let off = addr.wrapping_sub(self.base) as usize;
        if off < self.data.len() {
            Some(off)
        } else {
            None
        }
    }
}

impl Memory for FlatMemory {
    fn read8(&self, addr: u32) -> Result<u8, Trap> {
        let off = self.offset(addr).ok_or(Trap::LoadAccessFault)?;
        Ok(self.data[off])
    }

    fn read16(&self, addr: u32) -> Result<u16, Trap> {
        let off = self.offset(addr).ok_or(Trap::LoadAccessFault)?;
        if off + 2 > self.data.len() {
            return Err(Trap::LoadAccessFault);
        }
        Ok(u16::from_le_bytes([self.data[off], self.data[off + 1]]))
    }

    fn read32(&self, addr: u32) -> Result<u32, Trap> {
        let off = self.offset(addr).ok_or(Trap::LoadAccessFault)?;
        if off + 4 > self.data.len() {
            return Err(Trap::LoadAccessFault);
        }
        Ok(u32::from_le_bytes([
            self.data[off],
            self.data[off + 1],
            self.data[off + 2],
            self.data[off + 3],
        ]))
    }

    fn write8(&mut self, addr: u32, val: u8) -> Result<(), Trap> {
        let off = self.offset(addr).ok_or(Trap::StoreAccessFault)?;
        self.data[off] = val;
        Ok(())
    }

    fn write16(&mut self, addr: u32, val: u16) -> Result<(), Trap> {
        let off = self.offset(addr).ok_or(Trap::StoreAccessFault)?;
        if off + 2 > self.data.len() {
            return Err(Trap::StoreAccessFault);
        }
        let bytes = val.to_le_bytes();
        self.data[off] = bytes[0];
        self.data[off + 1] = bytes[1];
        Ok(())
    }

    fn write32(&mut self, addr: u32, val: u32) -> Result<(), Trap> {
        let off = self.offset(addr).ok_or(Trap::StoreAccessFault)?;
        if off + 4 > self.data.len() {
            return Err(Trap::StoreAccessFault);
        }
        let bytes = val.to_le_bytes();
        self.data[off] = bytes[0];
        self.data[off + 1] = bytes[1];
        self.data[off + 2] = bytes[2];
        self.data[off + 3] = bytes[3];
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_round_trip() {
        let mut mem = FlatMemory::new(1024, 0x8000_0000);
        mem.write32(0x8000_0000, 0xDEADBEEF).unwrap();
        assert_eq!(mem.read32(0x8000_0000).unwrap(), 0xDEADBEEF);
    }

    #[test]
    fn misaligned_access_supported() {
        let mut mem = FlatMemory::new(1024, 0x8000_0000);
        mem.write32(0x8000_0000, 0x04030201).unwrap();
        mem.write32(0x8000_0004, 0x08070605).unwrap();
        // Misaligned reads should succeed
        assert_eq!(mem.read16(0x8000_0001).unwrap(), 0x0302);
        assert_eq!(mem.read32(0x8000_0001).unwrap(), 0x05040302);
    }

    #[test]
    fn out_of_bounds() {
        let mem = FlatMemory::new(16, 0x8000_0000);
        assert_eq!(mem.read32(0x9000_0000), Err(Trap::LoadAccessFault));
    }

    #[test]
    fn byte_access_no_alignment() {
        let mut mem = FlatMemory::new(1024, 0x8000_0000);
        mem.write8(0x8000_0001, 0xAB).unwrap();
        assert_eq!(mem.read8(0x8000_0001).unwrap(), 0xAB);
    }
}
