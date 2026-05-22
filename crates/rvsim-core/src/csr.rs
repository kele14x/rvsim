use std::collections::HashMap;

use crate::trap::Trap;

pub const CSR_CYCLE: u16 = 0xC00;
pub const CSR_CYCLEH: u16 = 0xC80;
pub const CSR_INSTRET: u16 = 0xC02;
pub const CSR_INSTRETH: u16 = 0xC82;
pub const CSR_MHARTID: u16 = 0xF14;
pub const CSR_MSTATUS: u16 = 0x300;
pub const CSR_MISA: u16 = 0x301;
pub const CSR_MTVEC: u16 = 0x305;
pub const CSR_MEPC: u16 = 0x341;
pub const CSR_MCAUSE: u16 = 0x342;
pub const CSR_MTVAL: u16 = 0x343;
pub const CSR_MEDELEG: u16 = 0x302;
pub const CSR_MIDELEG: u16 = 0x303;
pub const CSR_MIE: u16 = 0x304;
pub const CSR_MIP: u16 = 0x344;
pub const CSR_MSCRATCH: u16 = 0x340;
pub const CSR_SATP: u16 = 0x180;
pub const CSR_STVEC: u16 = 0x105;
pub const CSR_PMPADDR0: u16 = 0x3B0;
pub const CSR_PMPCFG0: u16 = 0x3A0;

// S-mode CSRs
pub const CSR_SSTATUS: u16 = 0x100;
pub const CSR_SIE: u16 = 0x104;
pub const CSR_SSCRATCH: u16 = 0x140;
pub const CSR_SEPC: u16 = 0x141;
pub const CSR_SCAUSE: u16 = 0x142;
pub const CSR_STVAL: u16 = 0x143;
pub const CSR_SIP: u16 = 0x144;

// mstatus field bit positions
pub const MSTATUS_SIE_BIT: u32 = 1;
pub const MSTATUS_MIE_BIT: u32 = 3;
pub const MSTATUS_SPIE_BIT: u32 = 5;
pub const MSTATUS_MPIE_BIT: u32 = 7;
pub const MSTATUS_SPP_BIT: u32 = 8;
pub const MSTATUS_MPP_SHIFT: u32 = 11;
pub const MSTATUS_MPP_MASK: u32 = 0x3 << MSTATUS_MPP_SHIFT;
pub const MSTATUS_MPRV: u32 = 1 << 17;
pub const MSTATUS_SUM: u32 = 1 << 18;
pub const MSTATUS_MXR: u32 = 1 << 19;

// mip / mie bit positions
pub const MIP_SSIP_BIT: u32 = 1;
pub const MIP_MSIP_BIT: u32 = 3;
pub const MIP_STIP_BIT: u32 = 5;
pub const MIP_MTIP_BIT: u32 = 7;
pub const MIP_SEIP_BIT: u32 = 9;
pub const MIP_MEIP_BIT: u32 = 11;

pub const MIP_SSIP: u32 = 1 << MIP_SSIP_BIT;
pub const MIP_MSIP: u32 = 1 << MIP_MSIP_BIT;
pub const MIP_STIP: u32 = 1 << MIP_STIP_BIT;
pub const MIP_MTIP: u32 = 1 << MIP_MTIP_BIT;
pub const MIP_SEIP: u32 = 1 << MIP_SEIP_BIT;
pub const MIP_MEIP: u32 = 1 << MIP_MEIP_BIT;

/// Bits of mip that software (CSR instructions) may modify directly.
/// Hardware-driven bits (MTIP, MEIP, SEIP) are read-only from software.
pub const MIP_SW_WRITABLE_PUB: u32 = MIP_SSIP | MIP_MSIP | MIP_STIP;
const MIP_SW_WRITABLE: u32 = MIP_SW_WRITABLE_PUB;

// Convenience bitmasks for mstatus.MIE / mstatus.SIE
pub const MSTATUS_MIE: u32 = 1 << MSTATUS_MIE_BIT;
pub const MSTATUS_SIE: u32 = 1 << MSTATUS_SIE_BIT;

// sstatus is a masked view of mstatus; these bits are visible in S-mode:
// SIE(1), SPIE(5), UBE(6), SPP(8), VS(10:9), FS(14:13), XS(16:15), SUM(18), MXR(19), SD(31)
const SSTATUS_MASK: u32 = 0x800D_E762;

// sie / sip are masked views of mie / mip: SSIP/STIP/SEIP only.
const SIE_SIP_MASK: u32 = MIP_SSIP | MIP_STIP | MIP_SEIP;

pub struct CsrFile {
    regs: HashMap<u16, u32>,
}

impl Default for CsrFile {
    fn default() -> Self {
        Self::new()
    }
}

impl CsrFile {
    pub fn new() -> Self {
        let mut regs = HashMap::new();
        // misa: RV32I + M + A
        regs.insert(CSR_MISA, (1 << 30) | (1 << 12) | (1 << 8) | (1 << 0)); // MXL=1 (32-bit) | M | I | A
        regs.insert(CSR_MHARTID, 0);
        regs.insert(CSR_MSTATUS, 0);
        regs.insert(CSR_MTVEC, 0);
        regs.insert(CSR_MEPC, 0);
        regs.insert(CSR_MCAUSE, 0);
        regs.insert(CSR_MTVAL, 0);
        regs.insert(CSR_MEDELEG, 0);
        regs.insert(CSR_MIDELEG, 0);
        regs.insert(CSR_MIE, 0);
        regs.insert(CSR_MIP, 0);
        regs.insert(CSR_MSCRATCH, 0);
        regs.insert(CSR_SATP, 0);
        regs.insert(CSR_STVEC, 0);
        regs.insert(CSR_PMPADDR0, 0);
        regs.insert(CSR_PMPCFG0, 0);
        // S-mode CSRs
        regs.insert(CSR_SSCRATCH, 0);
        regs.insert(CSR_SEPC, 0);
        regs.insert(CSR_SCAUSE, 0);
        regs.insert(CSR_STVAL, 0);
        regs.insert(CSR_SIE, 0);
        regs.insert(CSR_SIP, 0);
        Self { regs }
    }

    fn is_read_only(addr: u16) -> bool {
        // CSRs with top 2 bits = 11 are read-only
        (addr >> 10) & 0x3 == 0x3
    }

    /// Minimum privilege level required to access this CSR (from addr bits [9:8])
    fn min_priv(addr: u16) -> u8 {
        match (addr >> 8) & 0x3 {
            0 => 0, // U-mode
            1 => 1, // S-mode
            2 => 3, // reserved, treat as M
            3 => 3, // M-mode
            _ => unreachable!(),
        }
    }

    pub fn read(&self, addr: u16, cycle: u64, instret: u64, priv_mode: u8) -> Result<u32, Trap> {
        // Privilege check
        if priv_mode < Self::min_priv(addr) {
            return Err(Trap::IllegalInstruction);
        }
        match addr {
            CSR_CYCLE => Ok(cycle as u32),
            CSR_CYCLEH => Ok((cycle >> 32) as u32),
            CSR_INSTRET => Ok(instret as u32),
            CSR_INSTRETH => Ok((instret >> 32) as u32),
            // sstatus is a masked view of mstatus
            CSR_SSTATUS => {
                let mstatus = self.regs.get(&CSR_MSTATUS).copied().unwrap_or(0);
                Ok(mstatus & SSTATUS_MASK)
            }
            // sie / sip are masked views of mie / mip
            CSR_SIE => Ok(self.regs.get(&CSR_MIE).copied().unwrap_or(0) & SIE_SIP_MASK),
            CSR_SIP => Ok(self.regs.get(&CSR_MIP).copied().unwrap_or(0) & SIE_SIP_MASK),
            _ => self.regs.get(&addr).copied().ok_or(Trap::IllegalInstruction),
        }
    }

    pub fn write(&mut self, addr: u16, val: u32, priv_mode: u8) -> Result<(), Trap> {
        // Privilege check
        if priv_mode < Self::min_priv(addr) {
            return Err(Trap::IllegalInstruction);
        }
        if Self::is_read_only(addr) {
            return Err(Trap::IllegalInstruction);
        }
        match addr {
            CSR_CYCLE | CSR_CYCLEH | CSR_INSTRET | CSR_INSTRETH => {
                Err(Trap::IllegalInstruction)
            }
            CSR_SSTATUS => {
                let e = self.regs.entry(CSR_MSTATUS).or_insert(0);
                *e = (*e & !SSTATUS_MASK) | (val & SSTATUS_MASK);
                Ok(())
            }
            // sie / sip write through to mie / mip, only the S-visible bits.
            CSR_SIE => {
                let e = self.regs.entry(CSR_MIE).or_insert(0);
                *e = (*e & !SIE_SIP_MASK) | (val & SIE_SIP_MASK);
                Ok(())
            }
            CSR_SIP => {
                // From S-mode, only SSIP is software-writable inside the S-visible mask.
                let e = self.regs.entry(CSR_MIP).or_insert(0);
                *e = (*e & !MIP_SSIP) | (val & MIP_SSIP);
                Ok(())
            }
            // mip: only SSIP/MSIP/STIP are software-writable. MTIP/MEIP/SEIP are HW-driven.
            CSR_MIP => {
                let e = self.regs.entry(CSR_MIP).or_insert(0);
                *e = (*e & !MIP_SW_WRITABLE) | (val & MIP_SW_WRITABLE);
                Ok(())
            }
            _ => {
                use std::collections::hash_map::Entry;
                if let Entry::Occupied(mut e) = self.regs.entry(addr) {
                    e.insert(val);
                    Ok(())
                } else {
                    Err(Trap::IllegalInstruction)
                }
            }
        }
    }

    /// Direct read of a register (bypasses privilege checks) — used internally by Hart
    pub fn read_raw(&self, addr: u16) -> u32 {
        self.regs.get(&addr).copied().unwrap_or(0)
    }

    /// Direct write of a register (bypasses privilege checks) — used internally by Hart
    pub fn write_raw(&mut self, addr: u16, val: u32) {
        self.regs.insert(addr, val);
    }

    /// Trap entry: save priv to xPP, copy xIE to xPIE, clear xIE.
    pub fn mstatus_trap_enter(&mut self, prev_priv: u8, ie_bit: u32, pie_bit: u32, pp_shift: u32, pp_mask: u32) {
        let mut v = self.read_raw(CSR_MSTATUS);
        v = (v & !pp_mask) | ((prev_priv as u32) << pp_shift);
        let ie = (v >> ie_bit) & 1;
        v = (v & !(1 << pie_bit)) | (ie << pie_bit);
        v &= !(1 << ie_bit);
        self.write_raw(CSR_MSTATUS, v);
    }

    /// Trap return: restore priv from xPP, copy xPIE to xIE, set xPIE=1, clear xPP.
    pub fn mstatus_trap_return(&mut self, ie_bit: u32, pie_bit: u32, pp_shift: u32, pp_mask: u32) -> u8 {
        let mut v = self.read_raw(CSR_MSTATUS);
        let priv_mode = ((v >> pp_shift) & (pp_mask >> pp_shift)) as u8;
        let pie = (v >> pie_bit) & 1;
        v = (v & !(1 << ie_bit)) | (pie << ie_bit);
        v |= 1 << pie_bit;
        v &= !pp_mask;
        self.write_raw(CSR_MSTATUS, v);
        priv_mode
    }
}
