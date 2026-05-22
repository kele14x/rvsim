use crate::csr::{
    CsrFile, CSR_MCAUSE, CSR_MEPC, CSR_MTVAL, CSR_MTVEC,
    CSR_MEDELEG, CSR_SCAUSE, CSR_SEPC, CSR_STVAL, CSR_STVEC,
    MSTATUS_SIE_BIT, MSTATUS_MIE_BIT, MSTATUS_SPIE_BIT, MSTATUS_MPIE_BIT,
    MSTATUS_SPP_BIT, MSTATUS_MPP_SHIFT, MSTATUS_MPP_MASK,
};
use crate::decode::decode;
use crate::execute::execute;
use crate::mem::Memory;
use crate::reg::RegFile;
use crate::trap::Trap;

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

    pub fn step(&mut self, mem: &mut dyn Memory) {
        let trap_pc = self.pc;

        let result = (|| -> Result<(), Trap> {
            let raw = mem.read32(self.pc)?;
            let inst = decode(raw)?;
            self.pc = self.pc.wrapping_add(4);
            execute(self, mem, inst)?;
            Ok(())
        })();

        if let Err(trap) = result {
            self.handle_trap(trap, trap_pc);
        } else {
            self.instret += 1;
        }

        self.cycle += 1;
    }

    fn handle_trap(&mut self, trap: Trap, trap_pc: u32) {
        let cause = trap.cause_code();
        let delegate_to_s = self.priv_mode < PRIV_M
            && (self.csrs.read_raw(CSR_MEDELEG) & (1 << cause)) != 0;

        if delegate_to_s {
            self.csrs.write_raw(CSR_SEPC, trap_pc);
            self.csrs.write_raw(CSR_SCAUSE, cause);
            self.csrs.write_raw(CSR_STVAL, 0);
            self.csrs.mstatus_trap_enter(
                self.priv_mode, MSTATUS_SIE_BIT, MSTATUS_SPIE_BIT,
                MSTATUS_SPP_BIT, 1 << MSTATUS_SPP_BIT,
            );
            self.priv_mode = PRIV_S;
            self.pc = self.csrs.read_raw(CSR_STVEC) & !0x3;
        } else {
            self.csrs.write_raw(CSR_MEPC, trap_pc);
            self.csrs.write_raw(CSR_MCAUSE, cause);
            self.csrs.write_raw(CSR_MTVAL, 0);
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
            self.step(mem);
        }
    }
}
