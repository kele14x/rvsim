//! Physical Memory Protection (PMP) checks.
//!
//! PMP is RISC-V's pre-MMU access control. The M-mode firmware programs
//! `pmpcfg{0..3}` and `pmpaddr{0..15}` to define up to 16 address ranges
//! and the R/W/X permissions S/U-mode (and optionally locked M-mode) has
//! over them. A failed check raises an access fault, *not* a page fault.
//!
//! Matching rules per entry, in index order (0 wins ties):
//!   A=OFF   never matches
//!   A=TOR   matches if `prev_pmpaddr*4 <= addr < pmpaddr*4`
//!   A=NA4   matches a single 4-byte region at `pmpaddr*4`
//!   A=NAPOT matches `[base, base+size)` where size and base are encoded
//!           in pmpaddr (size = 2^(trailing_ones+3) bytes, ≥ 8).
//!
//! Permission rules:
//!   - First match wins. If matched, R/W/X bits decide allow/deny.
//!   - In M-mode, entries with L=0 are skipped (M bypass). With L=1,
//!     the entry applies even to M-mode.
//!   - If no entry matches: M-mode allows, S/U-mode denies.

use crate::cpu::{Hart, PRIV_M};
use crate::csr::{
    CSR_PMPADDR0, CSR_PMPCFG0, PMP_A_MASK, PMP_A_NA4, PMP_A_NAPOT, PMP_A_OFF, PMP_A_SHIFT,
    PMP_A_TOR, PMP_L, PMP_NUM_ENTRIES, PMP_R, PMP_W, PMP_X,
};
use crate::mmu::AccessType;
use crate::trap::{Trap, TrapInfo};

/// Run the PMP check for a physical access.
///
/// `va` is included only so the caller can attribute `tval` to the original
/// virtual address — PMP itself only sees the post-translation `pa`.
pub fn check(hart: &Hart, pa: u32, va: u32, access: AccessType, eff_priv: u8) -> Result<(), TrapInfo> {
    let mut prev_addr: u32 = 0;
    for i in 0..PMP_NUM_ENTRIES {
        let cfg = read_cfg(hart, i);
        let addr = read_addr(hart, i);
        let a = (cfg & PMP_A_MASK) >> PMP_A_SHIFT;

        let matched = match a {
            PMP_A_OFF => false,
            PMP_A_TOR => {
                let lo = prev_addr.wrapping_shl(2);
                let hi = addr.wrapping_shl(2);
                lo <= pa && pa < hi
            }
            PMP_A_NA4 => {
                let base = addr.wrapping_shl(2);
                pa >= base && pa < base.wrapping_add(4)
            }
            PMP_A_NAPOT => {
                // size encoded as: pmpaddr = base[..] | 0 | (k-3) trailing 1s,
                // where region size = 2^k bytes (k ≥ 3). 64-bit math throughout
                // so the whole-space encoding (trail=31, size=16 GiB) works.
                let trail = addr.trailing_ones();
                let size_log2 = trail + 3;
                let size: u64 = 1u64 << size_log2;
                let mask: u64 = size - 1;
                let base = ((addr as u64) << 2) & !mask;
                let pa64 = pa as u64;
                pa64 >= base && pa64 < base + size
            }
            _ => false,
        };
        prev_addr = addr;

        if !matched {
            continue;
        }

        // Locked entries apply to M-mode; unlocked entries only constrain S/U.
        let locked = cfg & PMP_L != 0;
        if eff_priv == PRIV_M && !locked {
            return Ok(());
        }

        let perm_bit = match access {
            AccessType::Fetch => PMP_X,
            AccessType::Load => PMP_R,
            AccessType::Store => PMP_W,
        };
        return if cfg & perm_bit != 0 {
            Ok(())
        } else {
            Err(TrapInfo::new(access_fault(access), va))
        };
    }

    // No entry matched: M-mode allows, lower modes deny.
    if eff_priv == PRIV_M {
        Ok(())
    } else {
        Err(TrapInfo::new(access_fault(access), va))
    }
}

fn access_fault(access: AccessType) -> Trap {
    match access {
        AccessType::Fetch => Trap::InstructionAccessFault,
        AccessType::Load => Trap::LoadAccessFault,
        AccessType::Store => Trap::StoreAccessFault,
    }
}

/// Pull the cfg byte for entry `i` out of `pmpcfg{i/4}`.
fn read_cfg(hart: &Hart, i: usize) -> u8 {
    let reg = (i / 4) as u16;
    let shift = ((i % 4) * 8) as u32;
    let word = hart.csrs.read_raw(CSR_PMPCFG0 + reg);
    (word >> shift) as u8
}

fn read_addr(hart: &Hart, i: usize) -> u32 {
    hart.csrs.read_raw(CSR_PMPADDR0 + i as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{Hart, PRIV_S, PRIV_U};
    use crate::csr::{CSR_PMPADDR0, CSR_PMPCFG0};

    fn hart_with_cfg0(cfg: u32, addr0: u32) -> Hart {
        let mut h = Hart::new(0);
        h.csrs.write_raw(CSR_PMPCFG0, cfg);
        h.csrs.write_raw(CSR_PMPADDR0, addr0);
        h
    }

    #[test]
    fn mmode_allows_when_no_entries() {
        let h = Hart::new(0);
        // priv=M, no PMP entries → allow.
        check(&h, 0x1000, 0x1000, AccessType::Load, PRIV_M).unwrap();
    }

    #[test]
    fn umode_denies_when_no_entries() {
        let h = Hart::new(0);
        let err = check(&h, 0x1000, 0x1000, AccessType::Load, PRIV_U).unwrap_err();
        assert_eq!(err.cause, Trap::LoadAccessFault.cause_code());
    }

    #[test]
    fn mmode_bypasses_unlocked_entry() {
        // Entry 0: A=NAPOT, R=0,W=0,X=0, L=0, covers [0, 4 GiB).
        // For NAPOT covering whole space: pmpaddr = 0x7fff_ffff.
        let cfg = (PMP_A_NAPOT << PMP_A_SHIFT) as u32; // R/W/X = 0, L = 0
        let h = hart_with_cfg0(cfg, 0x7fff_ffff);
        // M-mode skips because L=0.
        check(&h, 0x1000, 0x1000, AccessType::Load, PRIV_M).unwrap();
    }

    #[test]
    fn mmode_locked_entry_applies() {
        let cfg = ((PMP_A_NAPOT << PMP_A_SHIFT) | PMP_L) as u32; // L=1, no perms
        let h = hart_with_cfg0(cfg, 0x7fff_ffff);
        let err = check(&h, 0x1000, 0x1000, AccessType::Load, PRIV_M).unwrap_err();
        assert_eq!(err.cause, Trap::LoadAccessFault.cause_code());
    }

    #[test]
    fn napot_grants_within_range() {
        // 4 KiB region at base 0x8000_0000:
        // pmpaddr = base>>2 | ((size>>3) - 1) → 0x2000_0000 | 0x1FF = 0x2000_01FF
        let cfg = ((PMP_A_NAPOT << PMP_A_SHIFT) | PMP_R | PMP_W | PMP_X) as u32;
        let h = hart_with_cfg0(cfg, 0x2000_01FF);
        check(&h, 0x8000_0000, 0, AccessType::Load, PRIV_U).unwrap();
        check(&h, 0x8000_0FFF, 0, AccessType::Store, PRIV_U).unwrap();
        let err = check(&h, 0x8000_1000, 0, AccessType::Load, PRIV_U).unwrap_err();
        assert_eq!(err.cause, Trap::LoadAccessFault.cause_code());
    }

    #[test]
    fn tor_uses_previous_addr_as_lower_bound() {
        // Entry 0: A=TOR with addr0=0x2000_0400 (PA 0x8000_1000), no perms, low=0.
        //   Matches PA in [0, 0x8000_1000).
        // Entry 1: A=TOR with addr1=0x2000_0800 (PA 0x8000_2000), R+W+X.
        //   Matches PA in [0x8000_1000, 0x8000_2000).
        let cfg0 = (PMP_A_TOR << PMP_A_SHIFT) as u32;
        let cfg1 = ((PMP_A_TOR << PMP_A_SHIFT) | PMP_R | PMP_W | PMP_X) as u32;
        let cfg = cfg0 | (cfg1 << 8);
        let mut h = Hart::new(0);
        h.csrs.write_raw(CSR_PMPCFG0, cfg);
        h.csrs.write_raw(CSR_PMPADDR0, 0x2000_0400);
        h.csrs.write_raw(CSR_PMPADDR0 + 1, 0x2000_0800);

        // PA inside entry 0 range, no perms → fault.
        let err = check(&h, 0x8000_0000, 0, AccessType::Load, PRIV_S).unwrap_err();
        assert_eq!(err.cause, Trap::LoadAccessFault.cause_code());
        // PA inside entry 1 range, full perms → allow.
        check(&h, 0x8000_1000, 0, AccessType::Load, PRIV_S).unwrap();
    }
}
