use crate::csr::{CsrFile, CSR_MCAUSE, CSR_MEPC, CSR_MTVAL, CSR_MTVEC};
use crate::decode::decode;
use crate::execute::execute;
use crate::mem::Memory;
use crate::reg::RegFile;
use crate::trap::Trap;

pub struct Hart {
    pub pc: u32,
    pub regs: RegFile,
    pub csrs: CsrFile,
    pub cycle: u64,
    pub instret: u64,
    /// Load-reserved address for LR/SC (None = no reservation)
    pub reservation: Option<u32>,
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
        }

        self.cycle += 1;
        self.instret += 1;
    }

    fn handle_trap(&mut self, trap: Trap, trap_pc: u32) {
        // Save exception PC
        self.csrs.write(CSR_MEPC, trap_pc).ok();
        // Save cause
        self.csrs.write(CSR_MCAUSE, trap.cause_code()).ok();
        // Save trap value (0 for now — could be faulting address for load/store)
        self.csrs.write(CSR_MTVAL, 0).ok();
        // Jump to trap vector
        let mtvec = self.csrs.read(CSR_MTVEC, self.cycle, self.instret).unwrap_or(0);
        // mtvec MODE: 0 = Direct (all traps go to BASE), 1 = Vectored
        let base = mtvec & !0x3;
        self.pc = base;
    }

    pub fn run(&mut self, mem: &mut dyn Memory, max_cycles: u64) {
        for _ in 0..max_cycles {
            self.step(mem);
        }
    }
}
