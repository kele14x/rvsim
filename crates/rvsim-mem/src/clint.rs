//! Core-Local Interruptor (CLINT) for a single hart.
//!
//! Register layout (relative to base):
//! - 0x0000: msip   (4 B)   — software interrupt pending; bit 0 only
//! - 0x4000: mtimecmp (8 B) — timer compare
//! - 0xBFF8: mtime    (8 B) — monotonic counter
//!
//! On RV32, `mtime`/`mtimecmp` are accessed as two 32-bit halves. We support
//! that explicitly. mtime is driven by [`Clint::tick`] from the simulation
//! loop (cycle-based, deterministic).

use std::cell::RefCell;

use rvsim_core::trap::Trap;

pub const CLINT_BASE: u32 = 0x0200_0000;
pub const CLINT_SIZE: u32 = 0x0001_0000;

const OFF_MSIP: u32 = 0x0000;
const OFF_MTIMECMP_LO: u32 = 0x4000;
const OFF_MTIMECMP_HI: u32 = 0x4004;
const OFF_MTIME_LO: u32 = 0xBFF8;
const OFF_MTIME_HI: u32 = 0xBFFC;

struct State {
    mtime: u64,
    mtimecmp: u64,
    msip: u32,
}

pub struct Clint {
    state: RefCell<State>,
}

impl Default for Clint {
    fn default() -> Self {
        Self::new()
    }
}

impl Clint {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(State {
                mtime: 0,
                // mtimecmp = max ⇒ MTIP starts deasserted (matches reset behavior
                // expected by OpenSBI / Linux before they program it).
                mtimecmp: u64::MAX,
                msip: 0,
            }),
        }
    }

    /// Advance the monotonic counter. Called once per simulator tick.
    pub fn tick(&mut self, cycle: u64) {
        self.state.get_mut().mtime = cycle;
    }

    /// Is the M-mode software-interrupt line asserted? (drives mip.MSIP)
    pub fn msip_pending(&self) -> bool {
        self.state.borrow().msip & 1 != 0
    }

    /// Is the M-mode timer-interrupt line asserted? (drives mip.MTIP)
    pub fn mtip_pending(&self) -> bool {
        let s = self.state.borrow();
        s.mtime >= s.mtimecmp
    }

    pub fn read32(&self, offset: u32) -> Result<u32, Trap> {
        let s = self.state.borrow();
        match offset {
            OFF_MSIP => Ok(s.msip),
            OFF_MTIMECMP_LO => Ok(s.mtimecmp as u32),
            OFF_MTIMECMP_HI => Ok((s.mtimecmp >> 32) as u32),
            OFF_MTIME_LO => Ok(s.mtime as u32),
            OFF_MTIME_HI => Ok((s.mtime >> 32) as u32),
            _ => Err(Trap::LoadAccessFault),
        }
    }

    pub fn write32(&self, offset: u32, val: u32) -> Result<(), Trap> {
        let mut s = self.state.borrow_mut();
        match offset {
            OFF_MSIP => {
                s.msip = val & 1;
                Ok(())
            }
            OFF_MTIMECMP_LO => {
                s.mtimecmp = (s.mtimecmp & 0xFFFF_FFFF_0000_0000) | (val as u64);
                Ok(())
            }
            OFF_MTIMECMP_HI => {
                s.mtimecmp = (s.mtimecmp & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32);
                Ok(())
            }
            OFF_MTIME_LO => {
                s.mtime = (s.mtime & 0xFFFF_FFFF_0000_0000) | (val as u64);
                Ok(())
            }
            OFF_MTIME_HI => {
                s.mtime = (s.mtime & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32);
                Ok(())
            }
            _ => Err(Trap::StoreAccessFault),
        }
    }

    // Byte/halfword accesses on CLINT registers aren't used by OpenSBI/Linux,
    // but provide them so we don't accidentally raise faults on stray probes.
    pub fn read8(&self, offset: u32) -> Result<u8, Trap> {
        let word = self.read32(offset & !0x3)?;
        Ok(((word >> ((offset & 0x3) * 8)) & 0xFF) as u8)
    }

    pub fn read16(&self, offset: u32) -> Result<u16, Trap> {
        let word = self.read32(offset & !0x3)?;
        Ok(((word >> ((offset & 0x2) * 8)) & 0xFFFF) as u16)
    }

    pub fn write8(&self, offset: u32, val: u8) -> Result<(), Trap> {
        let aligned = offset & !0x3;
        let shift = (offset & 0x3) * 8;
        let cur = self.read32(aligned).unwrap_or(0);
        let new = (cur & !(0xFF << shift)) | ((val as u32) << shift);
        self.write32(aligned, new)
    }

    pub fn write16(&self, offset: u32, val: u16) -> Result<(), Trap> {
        let aligned = offset & !0x3;
        let shift = (offset & 0x2) * 8;
        let cur = self.read32(aligned).unwrap_or(0);
        let new = (cur & !(0xFFFF << shift)) | ((val as u32) << shift);
        self.write32(aligned, new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtime_split_access() {
        let mut clint = Clint::new();
        clint.tick(0x1_0000_0001);
        assert_eq!(clint.read32(OFF_MTIME_LO).unwrap(), 0x0000_0001);
        assert_eq!(clint.read32(OFF_MTIME_HI).unwrap(), 0x0000_0001);
    }

    #[test]
    fn mtimecmp_split_write_then_read() {
        let clint = Clint::new();
        clint.write32(OFF_MTIMECMP_LO, 0xCAFE_BABE).unwrap();
        clint.write32(OFF_MTIMECMP_HI, 0xDEAD_BEEF).unwrap();
        assert_eq!(clint.read32(OFF_MTIMECMP_LO).unwrap(), 0xCAFE_BABE);
        assert_eq!(clint.read32(OFF_MTIMECMP_HI).unwrap(), 0xDEAD_BEEF);
    }

    #[test]
    fn mtip_fires_when_mtime_reaches_mtimecmp() {
        let mut clint = Clint::new();
        // Program mtimecmp = 100
        clint.write32(OFF_MTIMECMP_LO, 100).unwrap();
        clint.write32(OFF_MTIMECMP_HI, 0).unwrap();
        clint.tick(50);
        assert!(!clint.mtip_pending());
        clint.tick(100);
        assert!(clint.mtip_pending());
        clint.tick(200);
        assert!(clint.mtip_pending());
    }

    #[test]
    fn msip_toggles() {
        let clint = Clint::new();
        assert!(!clint.msip_pending());
        clint.write32(OFF_MSIP, 1).unwrap();
        assert!(clint.msip_pending());
        clint.write32(OFF_MSIP, 0).unwrap();
        assert!(!clint.msip_pending());
    }
}
