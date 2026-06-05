use crate::csr::{
    CsrFile, CSR_MCAUSE, CSR_MCOUNTINHIBIT, CSR_MEPC, CSR_MIDELEG, CSR_MIE, CSR_MIP, CSR_MSTATUS,
    CSR_MTVAL, CSR_MTVEC, CSR_MEDELEG, CSR_SCAUSE, CSR_SEPC, CSR_STVAL, CSR_STVEC,
    MCOUNTINHIBIT_CY, MCOUNTINHIBIT_IR,
    MIP_MEIP_BIT, MIP_MSIP_BIT, MIP_MTIP_BIT, MIP_SEIP_BIT, MIP_SSIP_BIT, MIP_STIP_BIT,
    MIP_SW_WRITABLE_PUB,
    MSTATUS_MIE_BIT, MSTATUS_MIE, MSTATUS_MPIE_BIT, MSTATUS_MPP_MASK, MSTATUS_MPP_SHIFT,
    MSTATUS_SIE_BIT, MSTATUS_SIE, MSTATUS_SPIE_BIT, MSTATUS_SPP_BIT,
};
use crate::decode::{decode, expand_compressed};
use crate::execute::execute;
use crate::mem::Memory;
use crate::mmu::{translate, AccessType};
use crate::reg::{RegFile, FpRegFile};
use crate::trap::{Trap, TrapInfo};

/// Privilege levels
pub const PRIV_U: u8 = 0;
pub const PRIV_S: u8 = 1;
pub const PRIV_M: u8 = 3;

pub struct Hart {
    pub pc: u32,
    pub regs: RegFile,
    pub fregs: FpRegFile,
    pub csrs: CsrFile,
    pub cycle: u64,
    pub instret: u64,
    /// Load-reserved address for LR/SC (None = no reservation)
    pub reservation: Option<u32>,
    /// Current privilege mode (0=U, 1=S, 3=M)
    pub priv_mode: u8,
    /// Set during execute when this instruction wrote mcycle/minstret — the
    /// explicit write supersedes the implicit retire bump for that counter.
    /// Bit 0 = cycle, bit 1 = instret. Reset at the start of each step.
    pub counter_written: u8,
}

pub const COUNTER_WRITTEN_CYCLE: u8 = 1 << 0;
pub const COUNTER_WRITTEN_INSTRET: u8 = 1 << 1;

impl Hart {
    pub fn new(start_pc: u32) -> Self {
        Self {
            pc: start_pc,
            regs: RegFile::new(),
            fregs: FpRegFile::new(),
            csrs: CsrFile::new(),
            cycle: 0,
            instret: 0,
            reservation: None,
            priv_mode: PRIV_M,
            counter_written: 0,
        }
    }

    /// Translate `va` with the given access type. Page-fault errors carry the VA as `tval`.
    pub fn translate(&self, mem: &dyn Memory, va: u32, access: AccessType) -> Result<u32, TrapInfo> {
        translate(self, mem, va, access)
    }

    /// Fetch the next instruction at `self.pc`, returning the (expanded) 32-bit
    /// form plus its width in bytes (2 for compressed, 4 for full). Halfwords
    /// are translated separately so a 4-byte instruction can straddle a page
    /// boundary — the second halfword may fault independently with the correct
    /// VA in tval.
    pub fn fetch_inst(&self, mem: &mut dyn Memory) -> Result<(u32, u8), TrapInfo> {
        let pa_lo = self.translate(mem, self.pc, AccessType::Fetch)?;
        let lo = mem
            .read16(pa_lo)
            .map_err(|_| TrapInfo::new(Trap::InstructionAccessFault, self.pc))?;

        if lo & 0x3 != 0x3 {
            let raw = expand_compressed(lo).map_err(|t| TrapInfo::new(t, lo as u32))?;
            return Ok((raw, 2));
        }

        let pc_hi = self.pc.wrapping_add(2);
        let pa_hi = self.translate(mem, pc_hi, AccessType::Fetch)?;
        let hi = mem
            .read16(pa_hi)
            .map_err(|_| TrapInfo::new(Trap::InstructionAccessFault, pc_hi))?;
        Ok((((hi as u32) << 16) | lo as u32, 4))
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
    pub fn load64(&self, mem: &mut dyn Memory, va: u32) -> Result<u64, TrapInfo> {
        let lo = self.load32(mem, va)? as u64;
        let hi = self.load32(mem, va.wrapping_add(4))? as u64;
        Ok((hi << 32) | lo)
    }
    pub fn store64(&self, mem: &mut dyn Memory, va: u32, val: u64) -> Result<(), TrapInfo> {
        self.store32(mem, va, val as u32)?;
        self.store32(mem, va.wrapping_add(4), (val >> 32) as u32)?;
        Ok(())
    }

    /// Drive one instruction. `pending_hw` carries mip bits set by hardware
    /// sources outside the CPU (CLINT MTIP/MSIP, PLIC SEIP). Software-writable
    /// bits already in `mip` are preserved.
    pub fn step(&mut self, mem: &mut dyn Memory, pending_hw: u32) {
        // Merge HW-driven mip bits with software-set bits.
        let sw = self.csrs.read_raw(CSR_MIP) & MIP_SW_WRITABLE_PUB;
        self.csrs.write_raw(CSR_MIP, sw | pending_hw);

        // mcountinhibit freezes counters when its bit is set.
        let inhibit = self.csrs.read_raw(CSR_MCOUNTINHIBIT);
        let tick_cycle = inhibit & MCOUNTINHIBIT_CY == 0;
        let tick_instret = inhibit & MCOUNTINHIBIT_IR == 0;

        // Clear the per-instruction counter-write flags.
        self.counter_written = 0;

        // Interrupts checked before fetch — taken between instructions.
        if let Some(code) = self.take_interrupt() {
            self.handle_trap(TrapInfo::interrupt(code), self.pc);
            if tick_cycle {
                self.cycle += 1;
            }
            return;
        }

        let trap_pc = self.pc;

        let result = (|| -> Result<(), TrapInfo> {
            let (raw, width) = self.fetch_inst(mem)?;
            let inst = decode(raw)?;
            self.pc = self.pc.wrapping_add(width as u32);
            execute(self, mem, inst, trap_pc)?;
            Ok(())
        })();

        if let Err(info) = result {
            self.handle_trap(info, trap_pc);
        } else if tick_instret && (self.counter_written & COUNTER_WRITTEN_INSTRET) == 0 {
            self.instret = self.instret.wrapping_add(1);
        }

        if tick_cycle && (self.counter_written & COUNTER_WRITTEN_CYCLE) == 0 {
            self.cycle = self.cycle.wrapping_add(1);
        }
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
        // Any trap entry invalidates an outstanding LR reservation.
        self.reservation = None;

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
