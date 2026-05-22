use crate::csr::{
    CsrFile, CSR_MCAUSE, CSR_MEPC, CSR_MSTATUS, CSR_MTVAL, CSR_MTVEC,
    CSR_MEDELEG, CSR_SCAUSE, CSR_SEPC, CSR_STVAL, CSR_STVEC,
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
        }

        self.cycle += 1;
        self.instret += 1;
    }

    fn handle_trap(&mut self, trap: Trap, trap_pc: u32) {
        let cause = trap.cause_code();

        // Check if this exception should be delegated to S-mode:
        // Delegate if the corresponding medeleg bit is set AND we're not already in M-mode
        let medeleg = self.csrs.read_raw(CSR_MEDELEG);
        let delegate_to_s = self.priv_mode < PRIV_M && (medeleg & (1 << cause)) != 0;

        if delegate_to_s {
            // Route to S-mode handler
            self.csrs.write_raw(CSR_SEPC, trap_pc);
            self.csrs.write_raw(CSR_SCAUSE, cause);
            self.csrs.write_raw(CSR_STVAL, 0);

            // Update mstatus: save current priv in SPP, save SIE in SPIE, clear SIE
            let mut mstatus = self.csrs.read_raw(CSR_MSTATUS);
            // SPP = previous privilege (bit 8: 0=U, 1=S)
            if self.priv_mode == PRIV_S {
                mstatus |= 1 << 8; // SPP = 1
            } else {
                mstatus &= !(1 << 8); // SPP = 0
            }
            // SPIE = SIE (bit 5 = bit 1)
            let sie = (mstatus >> 1) & 1;
            mstatus = (mstatus & !(1 << 5)) | (sie << 5);
            // Clear SIE
            mstatus &= !(1 << 1);
            self.csrs.write_raw(CSR_MSTATUS, mstatus);

            // Set privilege to S-mode
            self.priv_mode = PRIV_S;

            // Jump to stvec
            let stvec = self.csrs.read_raw(CSR_STVEC);
            let base = stvec & !0x3;
            self.pc = base;
        } else {
            // Route to M-mode handler
            self.csrs.write_raw(CSR_MEPC, trap_pc);
            self.csrs.write_raw(CSR_MCAUSE, cause);
            self.csrs.write_raw(CSR_MTVAL, 0);

            // Update mstatus: save current priv in MPP, save MIE in MPIE, clear MIE
            let mut mstatus = self.csrs.read_raw(CSR_MSTATUS);
            // MPP = previous privilege (bits 12:11)
            mstatus = (mstatus & !(0x3 << 11)) | ((self.priv_mode as u32) << 11);
            // MPIE = MIE (bit 7 = bit 3)
            let mie = (mstatus >> 3) & 1;
            mstatus = (mstatus & !(1 << 7)) | (mie << 7);
            // Clear MIE
            mstatus &= !(1 << 3);
            self.csrs.write_raw(CSR_MSTATUS, mstatus);

            // Set privilege to M-mode
            self.priv_mode = PRIV_M;

            // Jump to mtvec
            let mtvec = self.csrs.read_raw(CSR_MTVEC);
            let base = mtvec & !0x3;
            self.pc = base;
        }
    }

    pub fn run(&mut self, mem: &mut dyn Memory, max_cycles: u64) {
        for _ in 0..max_cycles {
            self.step(mem);
        }
    }
}
