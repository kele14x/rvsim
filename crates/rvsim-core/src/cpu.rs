use crate::csr::{
    CsrFile, CSR_MCAUSE, CSR_MEPC, CSR_MIDELEG, CSR_MIE, CSR_MIP, CSR_MSTATUS,
    CSR_MTVAL, CSR_MTVEC, CSR_MEDELEG, CSR_SCAUSE, CSR_SEPC, CSR_STVAL, CSR_STVEC,
    MIP_MEIP_BIT, MIP_MSIP_BIT, MIP_MTIP_BIT, MIP_SEIP_BIT, MIP_SSIP_BIT, MIP_STIP_BIT,
    MIP_SW_WRITABLE_PUB,
    MSTATUS_MIE_BIT, MSTATUS_MIE, MSTATUS_MPIE_BIT, MSTATUS_MPP_MASK, MSTATUS_MPP_SHIFT,
    MSTATUS_SIE_BIT, MSTATUS_SIE, MSTATUS_SPIE_BIT, MSTATUS_SPP_BIT,
};
use crate::decode::decode;
use crate::execute::execute;
use crate::mem::Memory;
use crate::mmu::{translate, AccessType};
use crate::reg::RegFile;
use crate::trap::{Trap, TrapInfo};

/// Privilege levels
pub const PRIV_U: u8 = 0;
pub const PRIV_S: u8 = 1;
pub const PRIV_M: u8 = 3;

pub struct Hart {
    pub pc: u32,
    pub regs: RegFile,
    pub csrs: CsrFile,
    pub cycle: u64,
    pub instret: u64,
    /// Load-reserved address for LR/SC (None = no reservation)
    pub reservation: Option<u32>,
    /// Current privilege mode (0=U, 1=S, 3=M)
    pub priv_mode: u8,
}

impl Hart {
    pub fn new(start_pc: u32) -> Self {
        Self {
            pc: start_pc,
            regs: RegFile::new(),
            csrs: CsrFile::new(),
            cycle: 0,
            instret: 0,
            reservation: None,
            priv_mode: PRIV_M,
        }
    }

    /// Translate `va` with the given access type. Page-fault errors carry the VA as `tval`.
    pub fn translate(&self, mem: &dyn Memory, va: u32, access: AccessType) -> Result<u32, TrapInfo> {
        translate(self, mem, va, access)
    }

    /// Fetch a 32-bit instruction at `self.pc`, translating through the MMU first.
    pub fn fetch32(&self, mem: &mut dyn Memory) -> Result<u32, TrapInfo> {
        let pa = self.translate(mem, self.pc, AccessType::Fetch)?;
        mem.read32(pa).map_err(|_| TrapInfo::new(Trap::InstructionAccessFault, self.pc))
    }

    pub fn load8(&self, mem: &mut dyn Memory, va: u32) -> Result<u8, TrapInfo> {
        let pa = self.translate(mem, va, AccessType::Load)?;
        Ok(mem.read8(pa)?)
    }
    pub fn load16(&self, mem: &mut dyn Memory, va: u32) -> Result<u16, TrapInfo> {
        let pa = self.translate(mem, va, AccessType::Load)?;
        Ok(mem.read16(pa)?)
    }
    pub fn load32(&self, mem: &mut dyn Memory, va: u32) -> Result<u32, TrapInfo> {
        let pa = self.translate(mem, va, AccessType::Load)?;
        Ok(mem.read32(pa)?)
    }
    pub fn store8(&self, mem: &mut dyn Memory, va: u32, val: u8) -> Result<(), TrapInfo> {
        let pa = self.translate(mem, va, AccessType::Store)?;
        Ok(mem.write8(pa, val)?)
    }
    pub fn store16(&self, mem: &mut dyn Memory, va: u32, val: u16) -> Result<(), TrapInfo> {
        let pa = self.translate(mem, va, AccessType::Store)?;
        Ok(mem.write16(pa, val)?)
    }
    pub fn store32(&self, mem: &mut dyn Memory, va: u32, val: u32) -> Result<(), TrapInfo> {
        let pa = self.translate(mem, va, AccessType::Store)?;
        Ok(mem.write32(pa, val)?)
    }

    /// Drive one instruction. `pending_hw` carries mip bits set by hardware
    /// sources outside the CPU (CLINT MTIP/MSIP, PLIC SEIP). Software-writable
    /// bits already in `mip` are preserved.
    pub fn step(&mut self, mem: &mut dyn Memory, pending_hw: u32) {
        // Merge HW-driven mip bits with software-set bits.
        let sw = self.csrs.read_raw(CSR_MIP) & MIP_SW_WRITABLE_PUB;
        self.csrs.write_raw(CSR_MIP, sw | pending_hw);

        // Interrupts checked before fetch — taken between instructions.
        if let Some(code) = self.take_interrupt() {
            self.handle_trap(TrapInfo::interrupt(code), self.pc);
            self.cycle += 1;
            return;
        }

        let trap_pc = self.pc;

        let result = (|| -> Result<(), TrapInfo> {
            let raw = self.fetch32(mem)?;
            let inst = decode(raw)?;
            self.pc = self.pc.wrapping_add(4);
            execute(self, mem, inst)?;
            Ok(())
        })();

        if let Err(info) = result {
            self.handle_trap(info, trap_pc);
        } else {
            self.instret += 1;
        }

        self.cycle += 1;
    }

    /// Compute the highest-priority deliverable interrupt, or `None`.
    /// Returns the bit index (0..31), not the full cause word.
    fn take_interrupt(&self) -> Option<u32> {
        let mip = self.csrs.read_raw(CSR_MIP);
        let mie = self.csrs.read_raw(CSR_MIE);
        let mideleg = self.csrs.read_raw(CSR_MIDELEG);
        let mstatus = self.csrs.read_raw(CSR_MSTATUS);

        let pending = mip & mie;
        if pending == 0 {
            return None;
        }

        let m_bits = pending & !mideleg;
        let s_bits = pending & mideleg;

        // M-mode interrupts: deliverable if priv < M, or (priv == M and MIE=1).
        let m_enabled = self.priv_mode < PRIV_M
            || (self.priv_mode == PRIV_M && (mstatus & MSTATUS_MIE) != 0);
        // S-mode interrupts: deliverable if priv < S, or (priv == S and SIE=1).
        // Never delivered while in M-mode.
        let s_enabled = self.priv_mode < PRIV_S
            || (self.priv_mode == PRIV_S && (mstatus & MSTATUS_SIE) != 0);

        // Priority order per privileged spec: MEI, MSI, MTI, SEI, SSI, STI.
        let candidates: [(u32, u32, bool); 6] = [
            (MIP_MEIP_BIT, m_bits, m_enabled),
            (MIP_MSIP_BIT, m_bits, m_enabled),
            (MIP_MTIP_BIT, m_bits, m_enabled),
            (MIP_SEIP_BIT, s_bits, s_enabled),
            (MIP_SSIP_BIT, s_bits, s_enabled),
            (MIP_STIP_BIT, s_bits, s_enabled),
        ];
        for (bit, bits, enabled) in candidates {
            if enabled && (bits & (1 << bit)) != 0 {
                return Some(bit);
            }
        }
        None
    }

    fn handle_trap(&mut self, info: TrapInfo, trap_pc: u32) {
        let cause = info.cause;
        let tval = info.tval;
        let idx = info.cause_index();

        // Delegation: interrupts use mideleg, exceptions use medeleg.
        let deleg = if info.is_interrupt() {
            self.csrs.read_raw(CSR_MIDELEG)
        } else {
            self.csrs.read_raw(CSR_MEDELEG)
        };
        let delegate_to_s = self.priv_mode < PRIV_M && (deleg & (1 << idx)) != 0;

        if delegate_to_s {
            self.csrs.write_raw(CSR_SEPC, trap_pc);
            self.csrs.write_raw(CSR_SCAUSE, cause);
            self.csrs.write_raw(CSR_STVAL, tval);
            self.csrs.mstatus_trap_enter(
                self.priv_mode, MSTATUS_SIE_BIT, MSTATUS_SPIE_BIT,
                MSTATUS_SPP_BIT, 1 << MSTATUS_SPP_BIT,
            );
            self.priv_mode = PRIV_S;
            self.pc = self.csrs.read_raw(CSR_STVEC) & !0x3;
        } else {
            self.csrs.write_raw(CSR_MEPC, trap_pc);
            self.csrs.write_raw(CSR_MCAUSE, cause);
            self.csrs.write_raw(CSR_MTVAL, tval);
            self.csrs.mstatus_trap_enter(
                self.priv_mode, MSTATUS_MIE_BIT, MSTATUS_MPIE_BIT,
                MSTATUS_MPP_SHIFT, MSTATUS_MPP_MASK,
            );
            self.priv_mode = PRIV_M;
            self.pc = self.csrs.read_raw(CSR_MTVEC) & !0x3;
        }
    }

    pub fn run(&mut self, mem: &mut dyn Memory, max_cycles: u64) {
        for _ in 0..max_cycles {
            self.step(mem, 0);
        }
    }
}
