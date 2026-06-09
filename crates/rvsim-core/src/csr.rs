use crate::trap::Trap;

// F-extension CSRs
pub const CSR_FFLAGS: u16 = 0x001;
pub const CSR_FRM: u16 = 0x002;
pub const CSR_FCSR: u16 = 0x003;

pub const CSR_CYCLE: u16 = 0xC00;
pub const CSR_TIME: u16 = 0xC01;
pub const CSR_CYCLEH: u16 = 0xC80;
pub const CSR_TIMEH: u16 = 0xC81;
pub const CSR_INSTRET: u16 = 0xC02;
pub const CSR_INSTRETH: u16 = 0xC82;
pub const CSR_MVENDORID: u16 = 0xF11;
pub const CSR_MARCHID: u16 = 0xF12;
pub const CSR_MIMPID: u16 = 0xF13;
pub const CSR_MHARTID: u16 = 0xF14;
pub const CSR_MCONFIGPTR: u16 = 0xF15;
pub const CSR_MSTATUS: u16 = 0x300;
pub const CSR_MISA: u16 = 0x301;
pub const CSR_MTVEC: u16 = 0x305;
pub const CSR_MCOUNTEREN: u16 = 0x306;
pub const CSR_MENVCFG: u16 = 0x30A;
pub const CSR_MSTATUSH: u16 = 0x310;
pub const CSR_MENVCFGH: u16 = 0x31A;
pub const CSR_MCOUNTINHIBIT: u16 = 0x320;
// mcountinhibit bits: setting a bit freezes the corresponding counter.
pub const MCOUNTINHIBIT_CY: u32 = 1 << 0;
pub const MCOUNTINHIBIT_IR: u32 = 1 << 2;
// Performance-counter CSRs (mhpmevent3..31, mhpmcounter3..31, *h variants).
// OpenSBI probes / clears these during init. We RAZ/WI them.
pub const CSR_MCYCLE: u16 = 0xB00;
pub const CSR_MINSTRET: u16 = 0xB02;
pub const CSR_MCYCLEH: u16 = 0xB80;
pub const CSR_MINSTRETH: u16 = 0xB82;
pub const CSR_MHPMEVENT_BASE: u16 = 0x323; // mhpmevent3..31 = 0x323..0x33F
pub const CSR_MHPMCOUNTER_BASE: u16 = 0xB03; // mhpmcounter3..31 = 0xB03..0xB1F
pub const CSR_MHPMCOUNTERH_BASE: u16 = 0xB83; // mhpmcounter3h..31h = 0xB83..0xB9F
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
pub const CSR_SCOUNTEREN: u16 = 0x106;
pub const CSR_SENVCFG: u16 = 0x10A;
// PMP CSRs (RV32: 4 cfg registers × 4 entries = 16 entries; 16 addr registers).
pub const CSR_PMPCFG0: u16 = 0x3A0;
pub const CSR_PMPCFG1: u16 = 0x3A1;
pub const CSR_PMPCFG2: u16 = 0x3A2;
pub const CSR_PMPCFG3: u16 = 0x3A3;
pub const CSR_PMPADDR0: u16 = 0x3B0;
pub const PMP_NUM_ENTRIES: usize = 16;
pub const PMP_R: u8 = 1 << 0;
pub const PMP_W: u8 = 1 << 1;
pub const PMP_X: u8 = 1 << 2;
pub const PMP_A_SHIFT: u8 = 3;
pub const PMP_A_MASK: u8 = 0x3 << PMP_A_SHIFT;
pub const PMP_A_OFF: u8 = 0;
pub const PMP_A_TOR: u8 = 1;
pub const PMP_A_NA4: u8 = 2;
pub const PMP_A_NAPOT: u8 = 3;
pub const PMP_L: u8 = 1 << 7;

// S-mode CSRs
pub const CSR_SSTATUS: u16 = 0x100;
pub const CSR_SIE: u16 = 0x104;
pub const CSR_SSCRATCH: u16 = 0x140;
pub const CSR_SEPC: u16 = 0x141;
pub const CSR_SCAUSE: u16 = 0x142;
pub const CSR_STVAL: u16 = 0x143;
pub const CSR_SIP: u16 = 0x144;

// Trigger module CSRs (debug) — RAZ/WI stubs (no triggers implemented).
pub const CSR_TSELECT: u16 = 0x7A0;
pub const CSR_TDATA1: u16 = 0x7A1;
pub const CSR_TDATA2: u16 = 0x7A2;
pub const CSR_TCONTROL: u16 = 0x7A5;

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
pub const MSTATUS_TVM: u32 = 1 << 20;
pub const MSTATUS_TW: u32 = 1 << 21;
pub const MSTATUS_TSR: u32 = 1 << 22;
pub const MSTATUS_FS_SHIFT: u32 = 13;
pub const MSTATUS_FS_MASK: u32 = 0x3 << MSTATUS_FS_SHIFT;
pub const MSTATUS_FS_DIRTY: u32 = 0x3 << MSTATUS_FS_SHIFT;
pub const MSTATUS_SD: u32 = 1 << 31;

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

fn mstatus_with_sd(v: u32) -> u32 {
    if v & MSTATUS_FS_MASK == MSTATUS_FS_DIRTY {
        v | MSTATUS_SD
    } else {
        v & !MSTATUS_SD
    }
}

pub struct CsrFile {
    regs: [u32; 4096],
}

impl Default for CsrFile {
    fn default() -> Self {
        Self::new()
    }
}

impl CsrFile {
    pub fn new() -> Self {
        let mut regs = [0u32; 4096];
        // misa: RV32IMACSU. Treated as WARL — writes are ignored (see write()).
        // S and U are required for OpenSBI: it probes misa to decide whether
        // to bring up an S-mode payload, and refuses to run the lottery /
        // coldboot path otherwise.
        regs[CSR_MISA as usize] =
            (1 << 30)   // MXL = 1 (32-bit)
                | (1 << 20) // U
                | (1 << 18) // S
                | (1 << 12) // M
                | (1 << 8)  // I
                | (1 << 5)  // F
                | (1 << 3)  // D
                | (1 << 2)  // C
                | (1 << 0); // A
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

    pub fn read(&self, addr: u16, cycle: u64, instret: u64, mtime: u64, priv_mode: u8) -> Result<u32, Trap> {
        if priv_mode < Self::min_priv(addr) {
            return Err(Trap::IllegalInstruction);
        }
        // mcounteren / scounteren gating for the U/S-visible counters.
        // M-mode always reads; S-mode needs mcounteren[bit]; U-mode needs both.
        if priv_mode != 3 {
            let counter_bit = match addr {
                CSR_CYCLE | CSR_CYCLEH => Some(0),     // CY
                CSR_TIME | CSR_TIMEH => Some(1),       // TM
                CSR_INSTRET | CSR_INSTRETH => Some(2), // IR
                _ => None,
            };
            if let Some(bit) = counter_bit {
                let mcen = self.regs[CSR_MCOUNTEREN as usize];
                if (mcen >> bit) & 1 == 0 {
                    return Err(Trap::IllegalInstruction);
                }
                if priv_mode == 0 {
                    let scen = self.regs[CSR_SCOUNTEREN as usize];
                    if (scen >> bit) & 1 == 0 {
                        return Err(Trap::IllegalInstruction);
                    }
                }
            }
        }
        match addr {
            CSR_CYCLE | CSR_MCYCLE => Ok(cycle as u32),
            CSR_CYCLEH | CSR_MCYCLEH => Ok((cycle >> 32) as u32),
            CSR_TIME => Ok(mtime as u32),
            CSR_TIMEH => Ok((mtime >> 32) as u32),
            CSR_INSTRET | CSR_MINSTRET => Ok(instret as u32),
            CSR_INSTRETH | CSR_MINSTRETH => Ok((instret >> 32) as u32),
            CSR_FCSR => Ok(self.regs[CSR_FCSR as usize] & 0xFF),
            CSR_FFLAGS => Ok(self.regs[CSR_FCSR as usize] & 0x1F),
            CSR_FRM => Ok((self.regs[CSR_FCSR as usize] >> 5) & 0x7),
            CSR_MSTATUS => Ok(mstatus_with_sd(self.regs[CSR_MSTATUS as usize])),
            CSR_SSTATUS => Ok(mstatus_with_sd(self.regs[CSR_MSTATUS as usize]) & SSTATUS_MASK),
            CSR_SIE => Ok(self.regs[CSR_MIE as usize] & SIE_SIP_MASK),
            CSR_SIP => Ok(self.regs[CSR_MIP as usize] & SIE_SIP_MASK),
            CSR_TSELECT | CSR_TDATA1 | CSR_TDATA2 | CSR_TCONTROL => Ok(0),
            _ => Ok(self.regs[addr as usize]),
        }
    }

    pub fn write(&mut self, addr: u16, val: u32, priv_mode: u8) -> Result<(), Trap> {
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
            CSR_MISA => Ok(()),
            CSR_FCSR => {
                self.regs[CSR_FCSR as usize] = val & 0xFF;
                Ok(())
            }
            CSR_FFLAGS => {
                self.regs[CSR_FCSR as usize] = (self.regs[CSR_FCSR as usize] & !0x1F) | (val & 0x1F);
                Ok(())
            }
            CSR_FRM => {
                self.regs[CSR_FCSR as usize] = (self.regs[CSR_FCSR as usize] & !0xE0) | ((val & 0x7) << 5);
                Ok(())
            }
            CSR_SSTATUS => {
                self.regs[CSR_MSTATUS as usize] = (self.regs[CSR_MSTATUS as usize] & !SSTATUS_MASK) | (val & SSTATUS_MASK);
                Ok(())
            }
            CSR_SIE => {
                self.regs[CSR_MIE as usize] = (self.regs[CSR_MIE as usize] & !SIE_SIP_MASK) | (val & SIE_SIP_MASK);
                Ok(())
            }
            CSR_SIP => {
                self.regs[CSR_MIP as usize] = (self.regs[CSR_MIP as usize] & !MIP_SSIP) | (val & MIP_SSIP);
                Ok(())
            }
            CSR_TSELECT | CSR_TDATA1 | CSR_TDATA2 | CSR_TCONTROL => Ok(()),
            CSR_MIP => {
                self.regs[CSR_MIP as usize] = (self.regs[CSR_MIP as usize] & !MIP_SW_WRITABLE) | (val & MIP_SW_WRITABLE);
                Ok(())
            }
            _ => {
                self.regs[addr as usize] = val;
                Ok(())
            }
        }
    }

    /// Direct read of a register (bypasses privilege checks) — used internally by Hart
    pub fn read_raw(&self, addr: u16) -> u32 {
        self.regs[addr as usize]
    }

    /// Direct write of a register (bypasses privilege checks) — used internally by Hart
    pub fn write_raw(&mut self, addr: u16, val: u32) {
        self.regs[addr as usize] = val;
    }

    /// Trap entry: save priv to xPP, copy xIE to xPIE, clear xIE.
    pub fn mstatus_trap_enter(&mut self, prev_priv: u8, ie_bit: u32, pie_bit: u32, pp_shift: u32, pp_mask: u32) {
        let mut v = self.regs[CSR_MSTATUS as usize];
        v = (v & !pp_mask) | ((prev_priv as u32) << pp_shift);
        let ie = (v >> ie_bit) & 1;
        v = (v & !(1 << pie_bit)) | (ie << pie_bit);
        v &= !(1 << ie_bit);
        self.regs[CSR_MSTATUS as usize] = v;
    }

    /// Trap return: restore priv from xPP, copy xPIE to xIE, set xPIE=1, clear xPP.
    /// Also clears MPRV when returning to a mode less privileged than M
    /// (privileged spec 1.12+).
    pub fn mstatus_trap_return(&mut self, ie_bit: u32, pie_bit: u32, pp_shift: u32, pp_mask: u32) -> u8 {
        let mut v = self.regs[CSR_MSTATUS as usize];
        let priv_mode = ((v >> pp_shift) & (pp_mask >> pp_shift)) as u8;
        let pie = (v >> pie_bit) & 1;
        v = (v & !(1 << ie_bit)) | (pie << ie_bit);
        v |= 1 << pie_bit;
        v &= !pp_mask;
        if priv_mode != 3 {
            v &= !MSTATUS_MPRV;
        }
        self.regs[CSR_MSTATUS as usize] = v;
        priv_mode
    }
}
