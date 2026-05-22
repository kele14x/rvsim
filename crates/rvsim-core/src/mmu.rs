//! Sv32 virtual-to-physical address translation.
//!
//! Translation is performed on every memory access (no TLB). M-mode bypasses
//! translation, as does `satp.MODE = Bare`. For loads/stores in M-mode with
//! `mstatus.MPRV = 1`, the effective privilege is `mstatus.MPP`.
//!
//! We do not auto-set the A/D bits — a missing A (or D on a store) raises a
//! page fault, matching what `rv32si-p-dirty` expects and how Linux's page
//! fault handler is written.

use crate::cpu::{Hart, PRIV_M};
use crate::csr::{CSR_MSTATUS, CSR_SATP, MSTATUS_MPP_MASK, MSTATUS_MPP_SHIFT, MSTATUS_MPRV, MSTATUS_MXR, MSTATUS_SUM};
use crate::mem::Memory;
use crate::trap::{Trap, TrapInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Fetch,
    Load,
    Store,
}

impl AccessType {
    fn page_fault(self) -> Trap {
        match self {
            AccessType::Fetch => Trap::InstructionPageFault,
            AccessType::Load => Trap::LoadPageFault,
            AccessType::Store => Trap::StorePageFault,
        }
    }
}

// PTE bit positions (Sv32)
const PTE_V: u32 = 1 << 0;
const PTE_R: u32 = 1 << 1;
const PTE_W: u32 = 1 << 2;
const PTE_X: u32 = 1 << 3;
const PTE_U: u32 = 1 << 4;
const PTE_A: u32 = 1 << 6;
const PTE_D: u32 = 1 << 7;

const PAGE_SIZE: u32 = 4096;
const LEVELS: i32 = 2;

/// Translate a virtual address to a physical address, walking the Sv32 page table.
///
/// Returns `Err(page_fault_on_va)` on any failure. The error's `tval` is always
/// the original virtual address (per Privileged spec).
pub fn translate(hart: &Hart, mem: &dyn Memory, va: u32, access: AccessType) -> Result<u32, TrapInfo> {
    let mstatus = hart.csrs.read_raw(CSR_MSTATUS);

    // Effective privilege: fetches always use current priv; loads/stores in
    // M-mode with MPRV=1 use MPP.
    let eff_priv = if access == AccessType::Fetch {
        hart.priv_mode
    } else if hart.priv_mode == PRIV_M && (mstatus & MSTATUS_MPRV) != 0 {
        ((mstatus & MSTATUS_MPP_MASK) >> MSTATUS_MPP_SHIFT) as u8
    } else {
        hart.priv_mode
    };

    let satp = hart.csrs.read_raw(CSR_SATP);
    let mode = (satp >> 31) & 1;

    // M-mode (effective) or Bare mode: identity translation.
    if eff_priv == PRIV_M || mode == 0 {
        return Ok(va);
    }

    // Sv32 walk.
    let vpn = [(va >> 12) & 0x3FF, (va >> 22) & 0x3FF]; // vpn[0], vpn[1]
    let root_ppn = satp & 0x003F_FFFF;
    let mut a = root_ppn.wrapping_mul(PAGE_SIZE);
    let mut i: i32 = LEVELS - 1;

    let pte;
    let level;
    loop {
        let pte_addr = a.wrapping_add(vpn[i as usize].wrapping_mul(4));
        let p = mem
            .read32(pte_addr)
            .map_err(|_| TrapInfo::new(access.page_fault(), va))?;

        // Invalid or reserved encoding (W=1, R=0).
        if (p & PTE_V) == 0 || ((p & PTE_R) == 0 && (p & PTE_W) != 0) {
            return Err(TrapInfo::new(access.page_fault(), va));
        }

        // Leaf?
        if (p & (PTE_R | PTE_X)) != 0 {
            pte = p;
            level = i;
            break;
        }

        // Non-leaf: descend.
        i -= 1;
        if i < 0 {
            return Err(TrapInfo::new(access.page_fault(), va));
        }
        // Next-level page table base = PPN field shifted up to byte address.
        // PPN[1] = bits 31:20 of PTE, PPN[0] = bits 19:10.
        let ppn = (p >> 10) & 0x003F_FFFF;
        a = ppn.wrapping_mul(PAGE_SIZE);
    }

    // Permission checks on the leaf PTE.
    match access {
        AccessType::Fetch => {
            if (pte & PTE_X) == 0 {
                return Err(TrapInfo::new(access.page_fault(), va));
            }
        }
        AccessType::Load => {
            // Loads need R; MXR allows X to substitute.
            let mxr = (mstatus & MSTATUS_MXR) != 0;
            let readable = (pte & PTE_R) != 0 || (mxr && (pte & PTE_X) != 0);
            if !readable {
                return Err(TrapInfo::new(access.page_fault(), va));
            }
        }
        AccessType::Store => {
            if (pte & PTE_W) == 0 {
                return Err(TrapInfo::new(access.page_fault(), va));
            }
        }
    }

    // U/S permission interaction.
    let is_user_page = (pte & PTE_U) != 0;
    if eff_priv == 0 {
        // U-mode: page must be U.
        if !is_user_page {
            return Err(TrapInfo::new(access.page_fault(), va));
        }
    } else {
        // S-mode (eff_priv == 1) accessing a U page:
        //   - Fetches always fault.
        //   - Loads/stores fault unless SUM is set.
        if is_user_page {
            if access == AccessType::Fetch {
                return Err(TrapInfo::new(access.page_fault(), va));
            }
            if (mstatus & MSTATUS_SUM) == 0 {
                return Err(TrapInfo::new(access.page_fault(), va));
            }
        }
    }

    // Megapage misalignment: leaf at level 1 with non-zero PPN[0].
    let ppn1 = (pte >> 20) & 0xFFF; // PTE bits 31:20
    let ppn0 = (pte >> 10) & 0x3FF; // PTE bits 19:10
    if level > 0 && ppn0 != 0 {
        return Err(TrapInfo::new(access.page_fault(), va));
    }

    // A/D bit check (fault rather than auto-set).
    if (pte & PTE_A) == 0 {
        return Err(TrapInfo::new(access.page_fault(), va));
    }
    if access == AccessType::Store && (pte & PTE_D) == 0 {
        return Err(TrapInfo::new(access.page_fault(), va));
    }

    // Assemble the physical address.
    let offset = va & 0xFFF;
    let pa = if level == 1 {
        // Megapage: PA[33:22] = ppn1, PA[21:12] = vpn[0]
        (ppn1 << 22) | (vpn[0] << 12) | offset
    } else {
        // 4 KiB page: PA[33:22] = ppn1, PA[21:12] = ppn0
        (ppn1 << 22) | (ppn0 << 12) | offset
    };
    Ok(pa)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{Hart, PRIV_S, PRIV_U};
    use crate::csr::{CSR_MSTATUS, CSR_SATP};
    use crate::mem::Memory;
    use crate::trap::Trap;

    // Tiny in-memory backing store for tests.
    struct VecMem(Vec<u8>);
    impl VecMem {
        fn new(size: usize) -> Self {
            Self(vec![0u8; size])
        }
    }
    impl Memory for VecMem {
        fn read8(&self, addr: u32) -> Result<u8, Trap> {
            self.0.get(addr as usize).copied().ok_or(Trap::LoadAccessFault)
        }
        fn read16(&self, addr: u32) -> Result<u16, Trap> {
            let a = addr as usize;
            if a + 2 > self.0.len() {
                return Err(Trap::LoadAccessFault);
            }
            Ok(u16::from_le_bytes([self.0[a], self.0[a + 1]]))
        }
        fn read32(&self, addr: u32) -> Result<u32, Trap> {
            let a = addr as usize;
            if a + 4 > self.0.len() {
                return Err(Trap::LoadAccessFault);
            }
            Ok(u32::from_le_bytes([self.0[a], self.0[a + 1], self.0[a + 2], self.0[a + 3]]))
        }
        fn write8(&mut self, addr: u32, val: u8) -> Result<(), Trap> {
            *self.0.get_mut(addr as usize).ok_or(Trap::StoreAccessFault)? = val;
            Ok(())
        }
        fn write16(&mut self, addr: u32, val: u16) -> Result<(), Trap> {
            let a = addr as usize;
            if a + 2 > self.0.len() {
                return Err(Trap::StoreAccessFault);
            }
            let b = val.to_le_bytes();
            self.0[a] = b[0];
            self.0[a + 1] = b[1];
            Ok(())
        }
        fn write32(&mut self, addr: u32, val: u32) -> Result<(), Trap> {
            let a = addr as usize;
            if a + 4 > self.0.len() {
                return Err(Trap::StoreAccessFault);
            }
            let b = val.to_le_bytes();
            self.0[a] = b[0];
            self.0[a + 1] = b[1];
            self.0[a + 2] = b[2];
            self.0[a + 3] = b[3];
            Ok(())
        }
    }

    fn pte(ppn: u32, flags: u32) -> u32 {
        (ppn << 10) | flags
    }

    /// Build a simple identity mapping for one 4 KiB page using two-level walk.
    /// Root table at PA 0x1000, second-level at PA 0x2000, target page at PA 0x3000.
    fn setup_one_page(perms: u32) -> (Hart, VecMem) {
        let mut mem = VecMem::new(0x10000);
        let leaf = pte(0x3, PTE_V | PTE_A | PTE_D | perms | PTE_U); // PPN = 3 → PA 0x3000
        let inner = pte(0x2, PTE_V); // points at second-level at PA 0x2000
        // Root PTE at index 0 of root table (covers VA 0..4 MiB)
        mem.write32(0x1000, inner).unwrap();
        // Second-level PTE at index 3 (covers VA 0x3000..0x4000)
        mem.write32(0x2000 + 3 * 4, leaf).unwrap();

        let mut hart = Hart::new(0);
        hart.priv_mode = PRIV_U;
        // satp: MODE=1, PPN=0x1 (root at PA 0x1000)
        hart.csrs.write_raw(CSR_SATP, (1 << 31) | 0x1);
        (hart, mem)
    }

    #[test]
    fn translate_u_mode_4k_page() {
        let (hart, mem) = setup_one_page(PTE_R | PTE_W | PTE_X);
        // VA 0x3000 → PA 0x3000 (identity).
        assert_eq!(translate(&hart, &mem, 0x3000, AccessType::Load).unwrap(), 0x3000);
        assert_eq!(translate(&hart, &mem, 0x3abc, AccessType::Store).unwrap(), 0x3abc);
        assert_eq!(translate(&hart, &mem, 0x3fff, AccessType::Fetch).unwrap(), 0x3fff);
    }

    #[test]
    fn translate_invalid_pte() {
        let (mut hart, mut mem) = setup_one_page(PTE_R);
        // Clear V on the leaf.
        let leaf = pte(0x3, PTE_A | PTE_D | PTE_R | PTE_U);
        mem.write32(0x2000 + 3 * 4, leaf).unwrap();
        hart.priv_mode = PRIV_U;
        let err = translate(&hart, &mem, 0x3000, AccessType::Load).unwrap_err();
        assert_eq!(err.cause, Trap::LoadPageFault.cause_code());
        assert_eq!(err.tval, 0x3000);
    }

    #[test]
    fn translate_reserved_encoding_w_without_r() {
        let (hart, mut mem) = setup_one_page(PTE_W);
        // Override leaf: W set, R clear, V set → reserved.
        let leaf = pte(0x3, PTE_V | PTE_A | PTE_D | PTE_W | PTE_U);
        mem.write32(0x2000 + 3 * 4, leaf).unwrap();
        let err = translate(&hart, &mem, 0x3000, AccessType::Load).unwrap_err();
        assert_eq!(err.cause, Trap::LoadPageFault.cause_code());
    }

    #[test]
    fn translate_store_without_d_faults() {
        let (hart, mut mem) = setup_one_page(PTE_R | PTE_W);
        // Drop D bit.
        let leaf = pte(0x3, PTE_V | PTE_A | PTE_R | PTE_W | PTE_U);
        mem.write32(0x2000 + 3 * 4, leaf).unwrap();
        // Load still succeeds (A is set).
        assert!(translate(&hart, &mem, 0x3000, AccessType::Load).is_ok());
        // Store faults because D=0.
        let err = translate(&hart, &mem, 0x3000, AccessType::Store).unwrap_err();
        assert_eq!(err.cause, Trap::StorePageFault.cause_code());
    }

    #[test]
    fn translate_no_a_faults() {
        let (hart, mut mem) = setup_one_page(PTE_R);
        // Drop A bit.
        let leaf = pte(0x3, PTE_V | PTE_R | PTE_U);
        mem.write32(0x2000 + 3 * 4, leaf).unwrap();
        let err = translate(&hart, &mem, 0x3000, AccessType::Load).unwrap_err();
        assert_eq!(err.cause, Trap::LoadPageFault.cause_code());
    }

    #[test]
    fn translate_smode_user_page_without_sum() {
        let (mut hart, mem) = setup_one_page(PTE_R);
        hart.priv_mode = PRIV_S;
        // SUM = 0, accessing U page → fault.
        let err = translate(&hart, &mem, 0x3000, AccessType::Load).unwrap_err();
        assert_eq!(err.cause, Trap::LoadPageFault.cause_code());

        // With SUM set, loads/stores work; fetches still fault.
        hart.csrs.write_raw(CSR_MSTATUS, MSTATUS_SUM);
        assert!(translate(&hart, &mem, 0x3000, AccessType::Load).is_ok());
        let err = translate(&hart, &mem, 0x3000, AccessType::Fetch).unwrap_err();
        assert_eq!(err.cause, Trap::InstructionPageFault.cause_code());
    }

    #[test]
    fn translate_mmode_bypasses() {
        let (mut hart, mem) = setup_one_page(0);
        hart.priv_mode = PRIV_M;
        // satp set but M-mode → identity, regardless of perms.
        assert_eq!(translate(&hart, &mem, 0x1234, AccessType::Load).unwrap(), 0x1234);
    }

    #[test]
    fn translate_bare_mode() {
        let mut hart = Hart::new(0);
        let mem = VecMem::new(0x100);
        hart.priv_mode = PRIV_U;
        // satp.MODE = 0 (Bare) → identity even in U-mode.
        hart.csrs.write_raw(CSR_SATP, 0);
        assert_eq!(translate(&hart, &mem, 0xdead_beef, AccessType::Load).unwrap(), 0xdead_beef);
    }

    #[test]
    fn translate_megapage() {
        // Root PTE is a leaf with R|W set → 4 MiB megapage.
        // ppn1=0x40 → PA[33:22]=0x40 → PA base 0x1000_0000 (fits in u32).
        let mut mem = VecMem::new(0x10000);
        let leaf = pte(0x40 << 10, PTE_V | PTE_A | PTE_D | PTE_R | PTE_W | PTE_U);
        mem.write32(0x1000, leaf).unwrap();

        let mut hart = Hart::new(0);
        hart.priv_mode = PRIV_U;
        hart.csrs.write_raw(CSR_SATP, (1 << 31) | 0x1);
        // VA 0x0000_1234 → PA 0x1000_1234 (low 22 bits passed through from VA).
        assert_eq!(translate(&hart, &mem, 0x1234, AccessType::Load).unwrap(), 0x1000_1234);
    }

    #[test]
    fn translate_misaligned_megapage_faults() {
        let mut mem = VecMem::new(0x10000);
        // ppn1=0x40, ppn0=1 (non-zero!) → misaligned megapage.
        let leaf = pte((0x40 << 10) | 1, PTE_V | PTE_A | PTE_D | PTE_R | PTE_W | PTE_U);
        mem.write32(0x1000, leaf).unwrap();
        let mut hart = Hart::new(0);
        hart.priv_mode = PRIV_U;
        hart.csrs.write_raw(CSR_SATP, (1 << 31) | 0x1);
        let err = translate(&hart, &mem, 0x1234, AccessType::Load).unwrap_err();
        assert_eq!(err.cause, Trap::LoadPageFault.cause_code());
    }
}
