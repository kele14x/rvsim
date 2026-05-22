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

        // Delivery target is decided per-bit by mideleg, not by which interrupt
        // it is. mideleg[i]=0 ⇒ delivered to M-mode; mideleg[i]=1 ⇒ delivered to
        // S-mode (provided the trap is currently taken at all).
        //
        // Enable rules:
        //   M-mode delivery: priv < M, or (priv == M and MIE=1).
        //   S-mode delivery: priv < S, or (priv == S and SIE=1).
        let m_enabled = self.priv_mode < PRIV_M
            || (self.priv_mode == PRIV_M && (mstatus & MSTATUS_MIE) != 0);
        let s_enabled = self.priv_mode < PRIV_S
            || (self.priv_mode == PRIV_S && (mstatus & MSTATUS_SIE) != 0);

        // Priority order per privileged spec: MEI, MSI, MTI, SEI, SSI, STI.
        let order = [
            MIP_MEIP_BIT, MIP_MSIP_BIT, MIP_MTIP_BIT,
            MIP_SEIP_BIT, MIP_SSIP_BIT, MIP_STIP_BIT,
        ];
        for bit in order {
            let mask = 1 << bit;
            if pending & mask == 0 {
                continue;
            }
            let to_s = mideleg & mask != 0;
            let enabled = if to_s { s_enabled } else { m_enabled };
            if enabled {
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
            self.pc = trap_target(self.csrs.read_raw(CSR_STVEC), info);
        } else {
            self.csrs.write_raw(CSR_MEPC, trap_pc);
            self.csrs.write_raw(CSR_MCAUSE, cause);
            self.csrs.write_raw(CSR_MTVAL, tval);
            self.csrs.mstatus_trap_enter(
                self.priv_mode, MSTATUS_MIE_BIT, MSTATUS_MPIE_BIT,
                MSTATUS_MPP_SHIFT, MSTATUS_MPP_MASK,
            );
            self.priv_mode = PRIV_M;
            self.pc = trap_target(self.csrs.read_raw(CSR_MTVEC), info);
        }
    }

}

/// Compute the trap entry PC from xtvec. MODE=0 (direct) sends everything to BASE;
/// MODE=1 (vectored) sends interrupts to BASE + 4*cause and exceptions to BASE.
fn trap_target(xtvec: u32, info: TrapInfo) -> u32 {
    let base = xtvec & !0x3;
    let mode = xtvec & 0x3;
    if mode == 1 && info.is_interrupt() {
        base + 4 * info.cause_index()
    } else {
        base
    }
}

impl Hart {
    pub fn run(&mut self, mem: &mut dyn Memory, max_cycles: u64) {
        for _ in 0..max_cycles {
            self.step(mem, 0);
        }
    }
}
