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
    EnvironmentCallFromMMode,
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
            Trap::EnvironmentCallFromMMode => 11,
        }
    }
}
