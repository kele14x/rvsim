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

/// A trap plus the value to write into `mtval` / `stval`.
/// For most traps `tval` is 0; for page faults it is the faulting virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrapInfo {
    pub trap: Trap,
    pub tval: u32,
}

impl TrapInfo {
    pub fn new(trap: Trap, tval: u32) -> Self {
        Self { trap, tval }
    }
}

impl From<Trap> for TrapInfo {
    fn from(trap: Trap) -> Self {
        Self { trap, tval: 0 }
    }
}
