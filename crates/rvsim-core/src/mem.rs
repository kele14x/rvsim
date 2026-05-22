use crate::trap::Trap;

pub trait Memory {
    fn read8(&self, addr: u32) -> Result<u8, Trap>;
    fn read16(&self, addr: u32) -> Result<u16, Trap>;
    fn read32(&self, addr: u32) -> Result<u32, Trap>;
    fn write8(&mut self, addr: u32, val: u8) -> Result<(), Trap>;
    fn write16(&mut self, addr: u32, val: u16) -> Result<(), Trap>;
    fn write32(&mut self, addr: u32, val: u32) -> Result<(), Trap>;
}
