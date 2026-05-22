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
        // misa: RV32I (bit 8 = 'I')
        regs.insert(CSR_MISA, (1 << 30) | (1 << 12) | (1 << 8)); // MXL=1 (32-bit) | M | I
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
        Self { regs }
    }

    fn is_read_only(addr: u16) -> bool {
        // CSRs with top 2 bits = 11 are read-only
        (addr >> 10) & 0x3 == 0x3
    }

    pub fn read(&self, addr: u16, cycle: u64, instret: u64) -> Result<u32, Trap> {
        match addr {
            CSR_CYCLE => Ok(cycle as u32),
            CSR_CYCLEH => Ok((cycle >> 32) as u32),
            CSR_INSTRET => Ok(instret as u32),
            CSR_INSTRETH => Ok((instret >> 32) as u32),
            _ => self.regs.get(&addr).copied().ok_or(Trap::IllegalInstruction),
        }
    }

    pub fn write(&mut self, addr: u16, val: u32) -> Result<(), Trap> {
        if Self::is_read_only(addr) {
            return Err(Trap::IllegalInstruction);
        }
        match addr {
            CSR_CYCLE | CSR_CYCLEH | CSR_INSTRET | CSR_INSTRETH => {
                Err(Trap::IllegalInstruction)
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
}
