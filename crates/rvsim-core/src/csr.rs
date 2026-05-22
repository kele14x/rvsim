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

// sstatus is a masked view of mstatus; these bits are visible in S-mode
// SIE(1), SPIE(5), UBE(6), SPP(8), VS(10:9), FS(14:13), XS(16:15), SUM(18), MXR(19), SD(31)
const SSTATUS_MASK: u32 = 0x800D_E762;

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
            // sstatus writes only affect the S-mode visible bits of mstatus
            CSR_SSTATUS => {
                let mstatus = self.regs.get(&CSR_MSTATUS).copied().unwrap_or(0);
                let new_mstatus = (mstatus & !SSTATUS_MASK) | (val & SSTATUS_MASK);
                self.regs.insert(CSR_MSTATUS, new_mstatus);
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
}
