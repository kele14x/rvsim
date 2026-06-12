#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trap {
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction,
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreAddressMisaligned,
    StoreAccessFault,
    EnvironmentCallFromUMode,
    EnvironmentCallFromSMode,
    EnvironmentCallFromMMode,
    InstructionPageFault,
    LoadPageFault,
    StorePageFault,
}

impl Trap {
    pub fn cause_code(self) -> u32 {
        match self {
            Trap::InstructionAddressMisaligned => 0,
            Trap::InstructionAccessFault => 1,
            Trap::IllegalInstruction => 2,
            Trap::Breakpoint => 3,
            Trap::LoadAddressMisaligned => 4,
            Trap::LoadAccessFault => 5,
            Trap::StoreAddressMisaligned => 6,
            Trap::StoreAccessFault => 7,
            Trap::EnvironmentCallFromUMode => 8,
            Trap::EnvironmentCallFromSMode => 9,
            Trap::EnvironmentCallFromMMode => 11,
            Trap::InstructionPageFault => 12,
            Trap::LoadPageFault => 13,
            Trap::StorePageFault => 15,
        }
    }
}

/// Combined cause + tval payload for trap entry.
///
/// `cause` is the full mcause/scause value, including bit 31 for interrupts.
/// `tval` is the value to write into mtval/stval (faulting VA for page faults,
/// 0 for most traps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrapInfo {
    pub cause: u32,
    pub tval: u32,
}

const INTERRUPT_BIT: u32 = 1 << 31;

impl TrapInfo {
    /// Build an exception trap (bit 31 clear). Convenience: takes a `Trap` and tval.
    pub fn new(trap: Trap, tval: u32) -> Self {
        Self {
            cause: trap.cause_code(),
            tval,
        }
    }

    /// Build an interrupt trap (bit 31 set). `code` is 0..31 (e.g. 7 for MTI).
    pub fn interrupt(code: u32) -> Self {
        Self {
            cause: INTERRUPT_BIT | code,
            tval: 0,
        }
    }

    pub fn is_interrupt(self) -> bool {
        (self.cause & INTERRUPT_BIT) != 0
    }

    /// The low 31 bits of `cause` — the exception code or interrupt index.
    pub fn cause_index(self) -> u32 {
        self.cause & !INTERRUPT_BIT
    }
}

impl From<Trap> for TrapInfo {
    fn from(trap: Trap) -> Self {
        Self::new(trap, 0)
    }
}
