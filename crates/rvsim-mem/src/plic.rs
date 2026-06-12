//! Platform-Level Interrupt Controller (PLIC) for a single hart.
//!
//! Implements the SiFive PLIC spec with 31 interrupt sources and 2 contexts
//! (context 0 = M-mode, context 1 = S-mode).
//!
//! Register layout (offsets relative to base 0x0C00_0000):
//! - 0x000004..0x00007C: source priority (source 1..31, 4 B each)
//! - 0x001000:           pending bits (1 word, bit per source, read-only)
//! - 0x002000:           context 0 (M) enable bits
//! - 0x002080:           context 1 (S) enable bits
//! - 0x200000:           context 0 priority threshold
//! - 0x200004:           context 0 claim (read) / complete (write)
//! - 0x201000:           context 1 priority threshold
//! - 0x201004:           context 1 claim/complete

use std::cell::RefCell;

use rvsim_core::trap::Trap;

pub const PLIC_BASE: u32 = 0x0C00_0000;
pub const PLIC_SIZE: u32 = 0x0400_0000;

const NUM_SOURCES: usize = 31;
const NUM_CONTEXTS: usize = 2;

const CTX_M: usize = 0;
const CTX_S: usize = 1;

struct State {
    priority: [u32; NUM_SOURCES + 1],
    pending: u32,
    enable: [u32; NUM_CONTEXTS],
    threshold: [u32; NUM_CONTEXTS],
    claimed: [u32; NUM_CONTEXTS],
}

pub struct Plic {
    state: RefCell<State>,
}

impl Default for Plic {
    fn default() -> Self {
        Self::new()
    }
}

impl Plic {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(State {
                priority: [0; NUM_SOURCES + 1],
                pending: 0,
                enable: [0; NUM_CONTEXTS],
                threshold: [0; NUM_CONTEXTS],
                claimed: [0; NUM_CONTEXTS],
            }),
        }
    }

    pub fn set_pending(&self, source: u32) {
        if source >= 1 && source <= NUM_SOURCES as u32 {
            self.state.borrow_mut().pending |= 1 << source;
        }
    }

    pub fn clear_pending(&self, source: u32) {
        if source >= 1 && source <= NUM_SOURCES as u32 {
            self.state.borrow_mut().pending &= !(1 << source);
        }
    }

    pub fn meip_pending(&self) -> bool {
        self.is_interrupted(CTX_M)
    }

    pub fn seip_pending(&self) -> bool {
        self.is_interrupted(CTX_S)
    }

    fn is_interrupted(&self, ctx: usize) -> bool {
        let s = self.state.borrow();
        let candidates = s.pending & s.enable[ctx] & !s.claimed[ctx] & !1;
        if candidates == 0 {
            return false;
        }
        for src in 1..=NUM_SOURCES {
            if candidates & (1 << src) != 0 && s.priority[src] > s.threshold[ctx] {
                return true;
            }
        }
        false
    }

    fn claim(&self, ctx: usize) -> u32 {
        let mut s = self.state.borrow_mut();
        let candidates = s.pending & s.enable[ctx] & !s.claimed[ctx] & !1;
        if candidates == 0 {
            return 0;
        }
        let mut best_src = 0u32;
        let mut best_pri = 0u32;
        for src in 1..=NUM_SOURCES {
            if candidates & (1 << src) != 0
                && s.priority[src] > s.threshold[ctx]
                && s.priority[src] > best_pri
            {
                best_pri = s.priority[src];
                best_src = src as u32;
            }
        }
        if best_src != 0 {
            s.pending &= !(1 << best_src);
            s.claimed[ctx] |= 1 << best_src;
        }
        best_src
    }

    fn complete(&self, ctx: usize, source: u32) {
        if source >= 1 && source <= NUM_SOURCES as u32 {
            self.state.borrow_mut().claimed[ctx] &= !(1 << source);
        }
    }

    fn ctx_from_offset(offset: u32) -> Option<usize> {
        let ctx = ((offset - 0x20_0000) / 0x1000) as usize;
        if ctx < NUM_CONTEXTS {
            Some(ctx)
        } else {
            None
        }
    }

    pub fn read32(&self, offset: u32) -> Result<u32, Trap> {
        let s = self.state.borrow();
        match offset {
            // Source priority (source 0 reads as 0)
            off if off < 0x80 => {
                let src = (off / 4) as usize;
                Ok(if src <= NUM_SOURCES {
                    s.priority[src]
                } else {
                    0
                })
            }
            // Pending bits
            0x1000 => Ok(s.pending),
            // Enable bits per context
            off @ 0x2000..=0x20FF => {
                let ctx = ((off - 0x2000) / 0x80) as usize;
                if ctx < NUM_CONTEXTS && (off & 0x7F) < 4 {
                    Ok(s.enable[ctx])
                } else {
                    Ok(0)
                }
            }
            // Context threshold / claim
            off @ 0x20_0000..=0x20_1FFF => {
                let reg = off & 0xFFF;
                if let Some(ctx) = Self::ctx_from_offset(off) {
                    match reg {
                        0x000 => Ok(s.threshold[ctx]),
                        0x004 => {
                            drop(s);
                            Ok(self.claim(ctx))
                        }
                        _ => Ok(0),
                    }
                } else {
                    Ok(0)
                }
            }
            _ => Ok(0), // RAZ
        }
    }

    pub fn write32(&self, offset: u32, val: u32) -> Result<(), Trap> {
        let mut s = self.state.borrow_mut();
        match offset {
            // Source priority (source 0 ignored)
            off if off < 0x80 => {
                let src = (off / 4) as usize;
                if (1..=NUM_SOURCES).contains(&src) {
                    s.priority[src] = val & 0x7; // 3-bit priority
                }
                Ok(())
            }
            // Pending bits are read-only
            0x1000 => Ok(()),
            // Enable bits per context
            off @ 0x2000..=0x20FF => {
                let ctx = ((off - 0x2000) / 0x80) as usize;
                if ctx < NUM_CONTEXTS && (off & 0x7F) < 4 {
                    s.enable[ctx] = val & !1; // bit 0 reserved
                }
                Ok(())
            }
            // Context threshold / complete
            off @ 0x20_0000..=0x20_1FFF => {
                let reg = off & 0xFFF;
                if let Some(ctx) = Self::ctx_from_offset(off) {
                    match reg {
                        0x000 => s.threshold[ctx] = val & 0x7,
                        0x004 => {
                            drop(s);
                            self.complete(ctx, val);
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
            _ => Ok(()), // WI
        }
    }

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
    fn no_interrupt_at_reset() {
        let plic = Plic::new();
        assert!(!plic.meip_pending());
        assert!(!plic.seip_pending());
    }

    #[test]
    fn pending_without_enable_does_not_interrupt() {
        let plic = Plic::new();
        plic.write32(0x04, 1).unwrap(); // source 1 priority = 1
        plic.set_pending(1);
        assert!(!plic.meip_pending()); // not enabled for context 0
    }

    #[test]
    fn enabled_pending_source_interrupts() {
        let plic = Plic::new();
        plic.write32(0x04, 1).unwrap(); // source 1 priority = 1
        plic.write32(0x2000, 0x02).unwrap(); // enable source 1 for ctx 0 (M)
        plic.set_pending(1);
        assert!(plic.meip_pending());
        assert!(!plic.seip_pending());
    }

    #[test]
    fn threshold_gates_interrupt() {
        let plic = Plic::new();
        plic.write32(0x04, 2).unwrap(); // source 1 priority = 2
        plic.write32(0x2000, 0x02).unwrap(); // enable source 1 for ctx 0
        plic.set_pending(1);
        assert!(plic.meip_pending());

        plic.write32(0x20_0000, 3).unwrap(); // threshold = 3 (> priority 2)
        assert!(!plic.meip_pending());

        plic.write32(0x20_0000, 1).unwrap(); // threshold = 1 (< priority 2)
        assert!(plic.meip_pending());
    }

    #[test]
    fn claim_returns_highest_priority_and_clears_pending() {
        let plic = Plic::new();
        plic.write32(0x04, 1).unwrap(); // source 1 priority = 1
        plic.write32(0x08, 3).unwrap(); // source 2 priority = 3
        plic.write32(0x2000, 0x06).unwrap(); // enable sources 1+2 for ctx 0
        plic.set_pending(1);
        plic.set_pending(2);

        let claimed = plic.read32(0x20_0004).unwrap(); // claim ctx 0
        assert_eq!(claimed, 2); // source 2 has higher priority
        assert!(plic.read32(0x1000).unwrap() & 0x04 == 0); // source 2 no longer pending
        assert!(plic.read32(0x1000).unwrap() & 0x02 != 0); // source 1 still pending
    }

    #[test]
    fn complete_allows_repend() {
        let plic = Plic::new();
        plic.write32(0x04, 1).unwrap(); // source 1 priority = 1
        plic.write32(0x2000, 0x02).unwrap(); // enable source 1 for ctx 0
        plic.set_pending(1);

        let claimed = plic.read32(0x20_0004).unwrap(); // claim
        assert_eq!(claimed, 1);
        assert!(!plic.meip_pending());

        plic.write32(0x20_0004, 1).unwrap(); // complete source 1
        plic.set_pending(1); // re-assert
        assert!(plic.meip_pending());
    }

    #[test]
    fn claim_returns_zero_when_nothing_pending() {
        let plic = Plic::new();
        assert_eq!(plic.read32(0x20_0004).unwrap(), 0);
    }

    #[test]
    fn s_mode_context() {
        let plic = Plic::new();
        plic.write32(0x04, 1).unwrap(); // source 1 priority = 1
        plic.write32(0x2080, 0x02).unwrap(); // enable source 1 for ctx 1 (S)
        plic.set_pending(1);
        assert!(!plic.meip_pending());
        assert!(plic.seip_pending());

        let claimed = plic.read32(0x20_1004).unwrap(); // claim ctx 1
        assert_eq!(claimed, 1);
        assert!(!plic.seip_pending());
    }

    #[test]
    fn pending_bits_read_only() {
        let plic = Plic::new();
        plic.set_pending(1);
        assert_eq!(plic.read32(0x1000).unwrap() & 0x02, 0x02);
        plic.write32(0x1000, 0).unwrap(); // should be ignored
        assert_eq!(plic.read32(0x1000).unwrap() & 0x02, 0x02);
    }

    #[test]
    fn claimed_source_not_reclaimable() {
        let plic = Plic::new();
        plic.write32(0x04, 1).unwrap(); // source 1 priority = 1
        plic.write32(0x2000, 0x02).unwrap(); // enable source 1 for ctx 0
        plic.set_pending(1);

        let claimed = plic.read32(0x20_0004).unwrap(); // claim
        assert_eq!(claimed, 1);

        // Re-pend while still claimed — must not be re-claimable
        plic.set_pending(1);
        assert!(!plic.meip_pending()); // EIP must not assert
        assert_eq!(plic.read32(0x20_0004).unwrap(), 0); // claim returns 0

        // After complete, re-pended source becomes claimable again
        plic.write32(0x20_0004, 1).unwrap(); // complete
        assert!(plic.meip_pending());
        assert_eq!(plic.read32(0x20_0004).unwrap(), 1);
    }
}
