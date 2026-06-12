use crate::trap::Trap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // R-type
    Add {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Sub {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Sll {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Slt {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Sltu {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Xor {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Srl {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Sra {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Or {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    And {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },

    // I-type (arithmetic)
    Addi {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Slti {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Sltiu {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Xori {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Ori {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Andi {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Slli {
        rd: u8,
        rs1: u8,
        shamt: u8,
    },
    Srli {
        rd: u8,
        rs1: u8,
        shamt: u8,
    },
    Srai {
        rd: u8,
        rs1: u8,
        shamt: u8,
    },

    // I-type (loads)
    Lb {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Lh {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Lw {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Lbu {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Lhu {
        rd: u8,
        rs1: u8,
        imm: i32,
    },

    // S-type (stores)
    Sb {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },
    Sh {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },
    Sw {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },

    // B-type (branches)
    Beq {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },
    Bne {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },
    Blt {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },
    Bge {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },
    Bltu {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },
    Bgeu {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },

    // U-type
    Lui {
        rd: u8,
        imm: u32,
    },
    Auipc {
        rd: u8,
        imm: u32,
    },

    // J-type
    Jal {
        rd: u8,
        imm: i32,
    },
    Jalr {
        rd: u8,
        rs1: u8,
        imm: i32,
    },

    // A extension
    LrW {
        rd: u8,
        rs1: u8,
    },
    ScW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    AmoswapW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    AmoaddW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    AmoxorW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    AmoandW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    AmoorW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    AmominW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    AmomaxW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    AmominuW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    AmomaxuW {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },

    // M extension
    Mul {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Mulh {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Mulhsu {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Mulhu {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Div {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Divu {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Rem {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    Remu {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },

    // F extension — loads/stores
    Flw {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Fsw {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },

    // F extension — arithmetic
    FaddS {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rm: u8,
    },
    FsubS {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rm: u8,
    },
    FmulS {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rm: u8,
    },
    FdivS {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rm: u8,
    },
    FsqrtS {
        rd: u8,
        rs1: u8,
        rm: u8,
    },

    // F extension — fused multiply-add
    FmaddS {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rs3: u8,
        rm: u8,
    },
    FmsubS {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rs3: u8,
        rm: u8,
    },
    FnmsubS {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rs3: u8,
        rm: u8,
    },
    FnmaddS {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rs3: u8,
        rm: u8,
    },

    // F extension — sign injection
    FsgnjS {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FsgnjnS {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FsgnjxS {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },

    // F extension — min/max
    FminS {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FmaxS {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },

    // F extension — compare (result to integer rd)
    FeqS {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FltS {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FleS {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },

    // F extension — classify
    FclassS {
        rd: u8,
        rs1: u8,
    },

    // F extension — convert int ↔ float
    FcvtWS {
        rd: u8,
        rs1: u8,
        rm: u8,
    },
    FcvtWuS {
        rd: u8,
        rs1: u8,
        rm: u8,
    },
    FcvtSW {
        rd: u8,
        rs1: u8,
        rm: u8,
    },
    FcvtSWu {
        rd: u8,
        rs1: u8,
        rm: u8,
    },

    // F extension — move/reinterpret
    FmvXW {
        rd: u8,
        rs1: u8,
    },
    FmvWX {
        rd: u8,
        rs1: u8,
    },

    // F/D — convert between S and D
    FcvtSD {
        rd: u8,
        rs1: u8,
        rm: u8,
    },
    FcvtDS {
        rd: u8,
        rs1: u8,
        rm: u8,
    },

    // D extension — loads/stores
    Fld {
        rd: u8,
        rs1: u8,
        imm: i32,
    },
    Fsd {
        rs1: u8,
        rs2: u8,
        imm: i32,
    },

    // D extension — arithmetic
    FaddD {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rm: u8,
    },
    FsubD {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rm: u8,
    },
    FmulD {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rm: u8,
    },
    FdivD {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rm: u8,
    },
    FsqrtD {
        rd: u8,
        rs1: u8,
        rm: u8,
    },

    // D extension — fused multiply-add
    FmaddD {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rs3: u8,
        rm: u8,
    },
    FmsubD {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rs3: u8,
        rm: u8,
    },
    FnmsubD {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rs3: u8,
        rm: u8,
    },
    FnmaddD {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rs3: u8,
        rm: u8,
    },

    // D extension — sign injection
    FsgnjD {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FsgnjnD {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FsgnjxD {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },

    // D extension — min/max
    FminD {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FmaxD {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },

    // D extension — compare (result to integer rd)
    FeqD {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FltD {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },
    FleD {
        rd: u8,
        rs1: u8,
        rs2: u8,
    },

    // D extension — classify
    FclassD {
        rd: u8,
        rs1: u8,
    },

    // D extension — convert int ↔ double
    FcvtWD {
        rd: u8,
        rs1: u8,
        rm: u8,
    },
    FcvtWuD {
        rd: u8,
        rs1: u8,
        rm: u8,
    },
    FcvtDW {
        rd: u8,
        rs1: u8,
        rm: u8,
    },
    FcvtDWu {
        rd: u8,
        rs1: u8,
        rm: u8,
    },

    // System
    Ecall,
    Ebreak,
    Mret,
    Sret,
    Wfi,
    SfenceVma,
    Fence,
    Csrrw {
        rd: u8,
        rs1: u8,
        csr: u16,
    },
    Csrrs {
        rd: u8,
        rs1: u8,
        csr: u16,
    },
    Csrrc {
        rd: u8,
        rs1: u8,
        csr: u16,
    },
    Csrrwi {
        rd: u8,
        uimm: u8,
        csr: u16,
    },
    Csrrsi {
        rd: u8,
        uimm: u8,
        csr: u16,
    },
    Csrrci {
        rd: u8,
        uimm: u8,
        csr: u16,
    },
}

fn rd(raw: u32) -> u8 {
    ((raw >> 7) & 0x1F) as u8
}

fn rs1(raw: u32) -> u8 {
    ((raw >> 15) & 0x1F) as u8
}

fn rs2(raw: u32) -> u8 {
    ((raw >> 20) & 0x1F) as u8
}

fn rs3(raw: u32) -> u8 {
    ((raw >> 27) & 0x1F) as u8
}

fn funct3(raw: u32) -> u32 {
    (raw >> 12) & 0x7
}

fn funct7(raw: u32) -> u32 {
    (raw >> 25) & 0x7F
}

fn imm_i(raw: u32) -> i32 {
    (raw as i32) >> 20
}

fn imm_s(raw: u32) -> i32 {
    let lo = (raw >> 7) & 0x1F;
    let hi = (raw >> 25) & 0x7F;
    let imm = (hi << 5) | lo;
    // Sign-extend from bit 11
    if raw & 0x8000_0000 != 0 {
        (imm | 0xFFFFF000) as i32
    } else {
        imm as i32
    }
}

fn imm_b(raw: u32) -> i32 {
    let bit12 = (raw >> 31) & 1;
    let bit11 = (raw >> 7) & 1;
    let bits10_5 = (raw >> 25) & 0x3F;
    let bits4_1 = (raw >> 8) & 0xF;
    let imm = (bit12 << 12) | (bit11 << 11) | (bits10_5 << 5) | (bits4_1 << 1);
    if bit12 != 0 {
        imm as i32 | !0x1FFF // sign extend from bit 12
    } else {
        imm as i32
    }
}

fn imm_u(raw: u32) -> u32 {
    raw & 0xFFFFF000
}

fn imm_j(raw: u32) -> i32 {
    let bit20 = (raw >> 31) & 1;
    let bits19_12 = (raw >> 12) & 0xFF;
    let bit11 = (raw >> 20) & 1;
    let bits10_1 = (raw >> 21) & 0x3FF;
    let imm = (bit20 << 20) | (bits19_12 << 12) | (bit11 << 11) | (bits10_1 << 1);
    if bit20 != 0 {
        imm as i32 | !0x1FFFFF // sign extend from bit 20
    } else {
        imm as i32
    }
}

pub fn decode(raw: u32) -> Result<Instruction, Trap> {
    let opcode = raw & 0x7F;

    match opcode {
        // LUI
        0b0110111 => Ok(Instruction::Lui {
            rd: rd(raw),
            imm: imm_u(raw),
        }),

        // AUIPC
        0b0010111 => Ok(Instruction::Auipc {
            rd: rd(raw),
            imm: imm_u(raw),
        }),

        // JAL
        0b1101111 => Ok(Instruction::Jal {
            rd: rd(raw),
            imm: imm_j(raw),
        }),

        // JALR (only funct3=000 is legal)
        0b1100111 => {
            if funct3(raw) != 0 {
                return Err(Trap::IllegalInstruction);
            }
            Ok(Instruction::Jalr {
                rd: rd(raw),
                rs1: rs1(raw),
                imm: imm_i(raw),
            })
        }

        // Branch
        0b1100011 => {
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let imm = imm_b(raw);
            match funct3(raw) {
                0b000 => Ok(Instruction::Beq {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                0b001 => Ok(Instruction::Bne {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                0b100 => Ok(Instruction::Blt {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                0b101 => Ok(Instruction::Bge {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                0b110 => Ok(Instruction::Bltu {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                0b111 => Ok(Instruction::Bgeu {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // Load
        0b0000011 => {
            let d = rd(raw);
            let r1 = rs1(raw);
            let imm = imm_i(raw);
            match funct3(raw) {
                0b000 => Ok(Instruction::Lb {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b001 => Ok(Instruction::Lh {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b010 => Ok(Instruction::Lw {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b100 => Ok(Instruction::Lbu {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b101 => Ok(Instruction::Lhu {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // Store
        0b0100011 => {
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let imm = imm_s(raw);
            match funct3(raw) {
                0b000 => Ok(Instruction::Sb {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                0b001 => Ok(Instruction::Sh {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                0b010 => Ok(Instruction::Sw {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // OP-IMM
        0b0010011 => {
            let d = rd(raw);
            let r1 = rs1(raw);
            let imm = imm_i(raw);
            match funct3(raw) {
                0b000 => Ok(Instruction::Addi {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b010 => Ok(Instruction::Slti {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b011 => Ok(Instruction::Sltiu {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b100 => Ok(Instruction::Xori {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b110 => Ok(Instruction::Ori {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b111 => Ok(Instruction::Andi {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b001 => {
                    // SLLI: funct7 must be 0 in RV32 (bit 25 set ⇒ shamt ≥ 32 ⇒ illegal).
                    if funct7(raw) != 0 {
                        return Err(Trap::IllegalInstruction);
                    }
                    Ok(Instruction::Slli {
                        rd: d,
                        rs1: r1,
                        shamt: (raw >> 20) as u8 & 0x1F,
                    })
                }
                0b101 => {
                    let shamt = (raw >> 20) as u8 & 0x1F;
                    match funct7(raw) {
                        0b0000000 => Ok(Instruction::Srli {
                            rd: d,
                            rs1: r1,
                            shamt,
                        }),
                        0b0100000 => Ok(Instruction::Srai {
                            rd: d,
                            rs1: r1,
                            shamt,
                        }),
                        _ => Err(Trap::IllegalInstruction),
                    }
                }
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // OP (R-type)
        0b0110011 => {
            let d = rd(raw);
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            match (funct3(raw), funct7(raw)) {
                (0b000, 0b0000000) => Ok(Instruction::Add {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b000, 0b0100000) => Ok(Instruction::Sub {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b001, 0b0000000) => Ok(Instruction::Sll {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b010, 0b0000000) => Ok(Instruction::Slt {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b011, 0b0000000) => Ok(Instruction::Sltu {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b100, 0b0000000) => Ok(Instruction::Xor {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b101, 0b0000000) => Ok(Instruction::Srl {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b101, 0b0100000) => Ok(Instruction::Sra {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b110, 0b0000000) => Ok(Instruction::Or {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b111, 0b0000000) => Ok(Instruction::And {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                // M extension (funct7 = 0b0000001)
                (0b000, 0b0000001) => Ok(Instruction::Mul {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b001, 0b0000001) => Ok(Instruction::Mulh {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b010, 0b0000001) => Ok(Instruction::Mulhsu {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b011, 0b0000001) => Ok(Instruction::Mulhu {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b100, 0b0000001) => Ok(Instruction::Div {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b101, 0b0000001) => Ok(Instruction::Divu {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b110, 0b0000001) => Ok(Instruction::Rem {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                (0b111, 0b0000001) => Ok(Instruction::Remu {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // AMO (A extension). On RV32 only funct3=010 (.W) is legal.
        0b0101111 => {
            if funct3(raw) != 0b010 {
                return Err(Trap::IllegalInstruction);
            }
            let d = rd(raw);
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let funct5 = (raw >> 27) & 0x1F;
            match funct5 {
                0b00010 => Ok(Instruction::LrW { rd: d, rs1: r1 }),
                0b00011 => Ok(Instruction::ScW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                0b00001 => Ok(Instruction::AmoswapW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                0b00000 => Ok(Instruction::AmoaddW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                0b00100 => Ok(Instruction::AmoxorW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                0b01100 => Ok(Instruction::AmoandW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                0b01000 => Ok(Instruction::AmoorW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                0b10000 => Ok(Instruction::AmominW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                0b10100 => Ok(Instruction::AmomaxW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                0b11000 => Ok(Instruction::AmominuW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                0b11100 => Ok(Instruction::AmomaxuW {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // MISC-MEM (FENCE)
        0b0001111 => Ok(Instruction::Fence),

        // SYSTEM
        0b1110011 => {
            let f3 = funct3(raw);
            if f3 == 0 {
                // Check funct7 for sfence.vma (funct7 = 0b0001001)
                if funct7(raw) == 0b0001001 {
                    return Ok(Instruction::SfenceVma);
                }
                match raw {
                    0x00000073 => Ok(Instruction::Ecall),
                    0x00100073 => Ok(Instruction::Ebreak),
                    0x30200073 => Ok(Instruction::Mret),
                    0x10200073 => Ok(Instruction::Sret),
                    0x10500073 => Ok(Instruction::Wfi),
                    _ => Err(Trap::IllegalInstruction),
                }
            } else {
                let d = rd(raw);
                let csr = ((raw >> 20) & 0xFFF) as u16;
                match f3 {
                    0b001 => Ok(Instruction::Csrrw {
                        rd: d,
                        rs1: rs1(raw),
                        csr,
                    }),
                    0b010 => Ok(Instruction::Csrrs {
                        rd: d,
                        rs1: rs1(raw),
                        csr,
                    }),
                    0b011 => Ok(Instruction::Csrrc {
                        rd: d,
                        rs1: rs1(raw),
                        csr,
                    }),
                    0b101 => Ok(Instruction::Csrrwi {
                        rd: d,
                        uimm: rs1(raw),
                        csr,
                    }),
                    0b110 => Ok(Instruction::Csrrsi {
                        rd: d,
                        uimm: rs1(raw),
                        csr,
                    }),
                    0b111 => Ok(Instruction::Csrrci {
                        rd: d,
                        uimm: rs1(raw),
                        csr,
                    }),
                    _ => Err(Trap::IllegalInstruction),
                }
            }
        }

        // LOAD-FP (FLW / FLD)
        0b0000111 => {
            let d = rd(raw);
            let r1 = rs1(raw);
            let imm = imm_i(raw);
            match funct3(raw) {
                0b010 => Ok(Instruction::Flw {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                0b011 => Ok(Instruction::Fld {
                    rd: d,
                    rs1: r1,
                    imm,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // STORE-FP (FSW / FSD)
        0b0100111 => {
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let imm = imm_s(raw);
            match funct3(raw) {
                0b010 => Ok(Instruction::Fsw {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                0b011 => Ok(Instruction::Fsd {
                    rs1: r1,
                    rs2: r2,
                    imm,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // FMADD
        0b1000011 => {
            let fmt = (raw >> 25) & 0x3;
            let d = rd(raw);
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let r3 = rs3(raw);
            let rm = funct3(raw) as u8;
            match fmt {
                0b00 => Ok(Instruction::FmaddS {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rs3: r3,
                    rm,
                }),
                0b01 => Ok(Instruction::FmaddD {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rs3: r3,
                    rm,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // FMSUB
        0b1000111 => {
            let fmt = (raw >> 25) & 0x3;
            let d = rd(raw);
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let r3 = rs3(raw);
            let rm = funct3(raw) as u8;
            match fmt {
                0b00 => Ok(Instruction::FmsubS {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rs3: r3,
                    rm,
                }),
                0b01 => Ok(Instruction::FmsubD {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rs3: r3,
                    rm,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // FNMSUB
        0b1001011 => {
            let fmt = (raw >> 25) & 0x3;
            let d = rd(raw);
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let r3 = rs3(raw);
            let rm = funct3(raw) as u8;
            match fmt {
                0b00 => Ok(Instruction::FnmsubS {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rs3: r3,
                    rm,
                }),
                0b01 => Ok(Instruction::FnmsubD {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rs3: r3,
                    rm,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // FNMADD
        0b1001111 => {
            let fmt = (raw >> 25) & 0x3;
            let d = rd(raw);
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let r3 = rs3(raw);
            let rm = funct3(raw) as u8;
            match fmt {
                0b00 => Ok(Instruction::FnmaddS {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rs3: r3,
                    rm,
                }),
                0b01 => Ok(Instruction::FnmaddD {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rs3: r3,
                    rm,
                }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // OP-FP (F/D extensions)
        0b1010011 => {
            let d = rd(raw);
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let rm = funct3(raw) as u8;
            match funct7(raw) {
                // F (single)
                0x00 => Ok(Instruction::FaddS {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rm,
                }),
                0x04 => Ok(Instruction::FsubS {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rm,
                }),
                0x08 => Ok(Instruction::FmulS {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rm,
                }),
                0x0C => Ok(Instruction::FdivS {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rm,
                }),
                0x2C => Ok(Instruction::FsqrtS { rd: d, rs1: r1, rm }),
                0x10 => match rm {
                    0b000 => Ok(Instruction::FsgnjS {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b001 => Ok(Instruction::FsgnjnS {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b010 => Ok(Instruction::FsgnjxS {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x14 => match rm {
                    0b000 => Ok(Instruction::FminS {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b001 => Ok(Instruction::FmaxS {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x50 => match rm {
                    0b010 => Ok(Instruction::FeqS {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b001 => Ok(Instruction::FltS {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b000 => Ok(Instruction::FleS {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x60 => match r2 {
                    0 => Ok(Instruction::FcvtWS { rd: d, rs1: r1, rm }),
                    1 => Ok(Instruction::FcvtWuS { rd: d, rs1: r1, rm }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x68 => match r2 {
                    0 => Ok(Instruction::FcvtSW { rd: d, rs1: r1, rm }),
                    1 => Ok(Instruction::FcvtSWu { rd: d, rs1: r1, rm }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x70 => match rm {
                    0b000 => Ok(Instruction::FmvXW { rd: d, rs1: r1 }),
                    0b001 => Ok(Instruction::FclassS { rd: d, rs1: r1 }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x78 => Ok(Instruction::FmvWX { rd: d, rs1: r1 }),
                // D (double)
                0x01 => Ok(Instruction::FaddD {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rm,
                }),
                0x05 => Ok(Instruction::FsubD {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rm,
                }),
                0x09 => Ok(Instruction::FmulD {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rm,
                }),
                0x0D => Ok(Instruction::FdivD {
                    rd: d,
                    rs1: r1,
                    rs2: r2,
                    rm,
                }),
                0x2D => Ok(Instruction::FsqrtD { rd: d, rs1: r1, rm }),
                0x11 => match rm {
                    0b000 => Ok(Instruction::FsgnjD {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b001 => Ok(Instruction::FsgnjnD {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b010 => Ok(Instruction::FsgnjxD {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x15 => match rm {
                    0b000 => Ok(Instruction::FminD {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b001 => Ok(Instruction::FmaxD {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x51 => match rm {
                    0b010 => Ok(Instruction::FeqD {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b001 => Ok(Instruction::FltD {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    0b000 => Ok(Instruction::FleD {
                        rd: d,
                        rs1: r1,
                        rs2: r2,
                    }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x61 => match r2 {
                    0 => Ok(Instruction::FcvtWD { rd: d, rs1: r1, rm }),
                    1 => Ok(Instruction::FcvtWuD { rd: d, rs1: r1, rm }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x69 => match r2 {
                    0 => Ok(Instruction::FcvtDW { rd: d, rs1: r1, rm }),
                    1 => Ok(Instruction::FcvtDWu { rd: d, rs1: r1, rm }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x71 => match rm {
                    0b001 => Ok(Instruction::FclassD { rd: d, rs1: r1 }),
                    _ => Err(Trap::IllegalInstruction),
                },
                // S ↔ D conversions
                0x20 => match r2 {
                    1 => Ok(Instruction::FcvtSD { rd: d, rs1: r1, rm }),
                    _ => Err(Trap::IllegalInstruction),
                },
                0x21 => match r2 {
                    0 => Ok(Instruction::FcvtDS { rd: d, rs1: r1, rm }),
                    _ => Err(Trap::IllegalInstruction),
                },
                _ => Err(Trap::IllegalInstruction),
            }
        }

        _ => Err(Trap::IllegalInstruction),
    }
}

/// Expand a 16-bit compressed (RVC) encoding into its canonical 32-bit equivalent.
/// The result is fed back through `decode()` so the execute path remains
/// width-agnostic. Returns `IllegalInstruction` for reserved encodings (e.g.
/// c.addi4spn with imm=0, c.lui with rd∈{0,2}, RV32 shamt[5]=1, etc.).
/// HINTs (legal but no-effect encodings) expand to their nop equivalents.
pub fn expand_compressed(half: u16) -> Result<u32, Trap> {
    let h = half as u32;
    let op = h & 0x3;
    let funct3 = (h >> 13) & 0x7;
    let rd_full = (h >> 7) & 0x1F; // bits 11:7
    let rs2_full = (h >> 2) & 0x1F; // bits 6:2
    let rd_p = ((h >> 2) & 0x7) + 8; // 3-bit compressed reg at bits 4:2
    let rs1_p = ((h >> 7) & 0x7) + 8; // 3-bit compressed reg at bits 9:7

    match (op, funct3) {
        // ===== Quadrant 0 =====
        (0b00, 0b000) => {
            // C.ADDI4SPN — addi rd', x2, nzuimm
            let nz = ((h >> 1) & 0x3C0)   // nzuimm[9:6] from inst[10:7]
                  | ((h >> 7) & 0x30)     // nzuimm[5:4] from inst[12:11]
                  | ((h >> 2) & 0x8)      // nzuimm[3]   from inst[5]
                  | ((h >> 4) & 0x4); // nzuimm[2]   from inst[6]
            if nz == 0 {
                return Err(Trap::IllegalInstruction);
            }
            Ok(enc_i(0x13, 0x0, rd_p, 2, nz as i32))
        }
        (0b00, 0b001) => {
            // C.FLD — fld rd', uimm(rs1')  (uimm[5:3|7:6])
            let uimm = ((h >> 7) & 0x38)    // uimm[5:3] from inst[12:10]
                    | ((h << 1) & 0xC0); // uimm[7:6] from inst[6:5]
            Ok(enc_i(0x07, 0x3, rd_p, rs1_p, uimm as i32))
        }
        (0b00, 0b010) => {
            // C.LW — lw rd', uimm(rs1')
            let uimm = ((h << 1) & 0x40)  // uimm[6] from inst[5]
                    | ((h >> 7) & 0x38)   // uimm[5:3] from inst[12:10]
                    | ((h >> 4) & 0x4); // uimm[2] from inst[6]
            Ok(enc_i(0x03, 0x2, rd_p, rs1_p, uimm as i32))
        }
        (0b00, 0b011) => {
            // C.FLW — flw rd', uimm(rs1')
            let uimm = ((h << 1) & 0x40) | ((h >> 7) & 0x38) | ((h >> 4) & 0x4);
            Ok(enc_i(0x07, 0x2, rd_p, rs1_p, uimm as i32))
        }
        (0b00, 0b101) => {
            // C.FSD — fsd rs2', uimm(rs1')  (uimm[5:3|7:6])
            let uimm = ((h >> 7) & 0x38) | ((h << 1) & 0xC0);
            Ok(enc_s(0x27, 0x3, rs1_p, rd_p, uimm as i32))
        }
        (0b00, 0b110) => {
            // C.SW — sw rs2', uimm(rs1')
            let uimm = ((h << 1) & 0x40) | ((h >> 7) & 0x38) | ((h >> 4) & 0x4);
            Ok(enc_s(0x23, 0x2, rs1_p, rd_p, uimm as i32))
        }
        (0b00, 0b111) => {
            // C.FSW — fsw rs2', uimm(rs1')
            let uimm = ((h << 1) & 0x40) | ((h >> 7) & 0x38) | ((h >> 4) & 0x4);
            Ok(enc_s(0x27, 0x2, rs1_p, rd_p, uimm as i32))
        }

        // ===== Quadrant 1 =====
        (0b01, 0b000) => {
            // C.ADDI / C.NOP — addi rd, rd, nzimm (rd=0 ⇒ NOP/HINT)
            let imm = sext(((h >> 7) & 0x20) | ((h >> 2) & 0x1F), 5);
            Ok(enc_i(0x13, 0x0, rd_full, rd_full, imm))
        }
        (0b01, 0b001) => {
            // C.JAL — jal x1, imm  (RV32 only; RV64 uses this slot for c.addiw)
            Ok(enc_j(0x6F, 1, compressed_j_imm(h)))
        }
        (0b01, 0b010) => {
            // C.LI — addi rd, x0, imm
            let imm = sext(((h >> 7) & 0x20) | ((h >> 2) & 0x1F), 5);
            Ok(enc_i(0x13, 0x0, rd_full, 0, imm))
        }
        (0b01, 0b011) => {
            if rd_full == 2 {
                // C.ADDI16SP — addi x2, x2, nzimm
                let bits = ((h >> 3) & 0x200)  // imm[9]   from inst[12]
                        | ((h << 4) & 0x180)   // imm[8:7] from inst[4:3]
                        | ((h << 1) & 0x40)    // imm[6]   from inst[5]
                        | ((h << 3) & 0x20)    // imm[5]   from inst[2]
                        | ((h >> 2) & 0x10); // imm[4]   from inst[6]
                if bits == 0 {
                    return Err(Trap::IllegalInstruction);
                }
                Ok(enc_i(0x13, 0x0, 2, 2, sext(bits, 9)))
            } else if rd_full != 0 {
                // C.LUI — lui rd, nzimm
                let bits = ((h << 5) & 0x20000)   // nzimm[17] from inst[12]
                        | ((h << 10) & 0x1F000); // nzimm[16:12] from inst[6:2]
                if bits == 0 {
                    return Err(Trap::IllegalInstruction);
                }
                Ok(enc_u(0x37, rd_full, sext(bits, 17) as u32))
            } else {
                Err(Trap::IllegalInstruction)
            }
        }
        (0b01, 0b100) => {
            let funct2 = (h >> 10) & 0x3;
            match funct2 {
                0b00 | 0b01 => {
                    // C.SRLI (00) / C.SRAI (01) — shifts on rd' by shamt
                    let shamt = ((h >> 7) & 0x20) | ((h >> 2) & 0x1F);
                    if shamt & 0x20 != 0 {
                        return Err(Trap::IllegalInstruction); // RV32: shamt[5] must be 0
                    }
                    let funct7 = if funct2 == 0 { 0x00 } else { 0x20 };
                    Ok(enc_r(0x13, 0x5, funct7, rs1_p, rs1_p, shamt))
                }
                0b10 => {
                    // C.ANDI — andi rd', rd', imm
                    let imm = sext(((h >> 7) & 0x20) | ((h >> 2) & 0x1F), 5);
                    Ok(enc_i(0x13, 0x7, rs1_p, rs1_p, imm))
                }
                _ => {
                    // funct2 = 11: C.SUB / C.XOR / C.OR / C.AND
                    if (h >> 12) & 1 != 0 {
                        // bit 12 = 1 is RV64-only (c.subw/c.addw) — reserved here
                        return Err(Trap::IllegalInstruction);
                    }
                    let (f3, f7) = match (h >> 5) & 0x3 {
                        0b00 => (0x0, 0x20), // SUB
                        0b01 => (0x4, 0x00), // XOR
                        0b10 => (0x6, 0x00), // OR
                        _ => (0x7, 0x00),    // AND
                    };
                    Ok(enc_r(0x33, f3, f7, rs1_p, rs1_p, rd_p))
                }
            }
        }
        (0b01, 0b101) => {
            // C.J — jal x0, imm
            Ok(enc_j(0x6F, 0, compressed_j_imm(h)))
        }
        (0b01, 0b110) | (0b01, 0b111) => {
            // C.BEQZ / C.BNEZ — branch rs1' vs x0
            let bits = ((h >> 4) & 0x100)   // imm[8]
                    | ((h << 1) & 0xC0)     // imm[7:6]
                    | ((h << 3) & 0x20)     // imm[5]
                    | ((h >> 7) & 0x18)     // imm[4:3]
                    | ((h >> 2) & 0x6); // imm[2:1]
            let imm = sext(bits, 8);
            let f3 = if funct3 == 0b110 { 0x0 } else { 0x1 };
            Ok(enc_b(0x63, f3, rs1_p, 0, imm))
        }

        // ===== Quadrant 2 =====
        (0b10, 0b000) => {
            // C.SLLI — slli rd, rd, shamt (rd=0 ⇒ HINT)
            let shamt = ((h >> 7) & 0x20) | ((h >> 2) & 0x1F);
            if shamt & 0x20 != 0 {
                return Err(Trap::IllegalInstruction);
            }
            Ok(enc_r(0x13, 0x1, 0x00, rd_full, rd_full, shamt))
        }
        (0b10, 0b001) => {
            // C.FLDSP — fld rd, uimm(x2)  (uimm[5|4:3|8:6])
            let uimm = ((h >> 7) & 0x20)    // uimm[5]
                    | ((h >> 2) & 0x18)     // uimm[4:3]
                    | ((h << 4) & 0x1C0); // uimm[8:6]
            Ok(enc_i(0x07, 0x3, rd_full, 2, uimm as i32))
        }
        (0b10, 0b010) => {
            // C.LWSP — lw rd, uimm(x2)
            if rd_full == 0 {
                return Err(Trap::IllegalInstruction);
            }
            let uimm = ((h >> 7) & 0x20)   // uimm[5]
                    | ((h >> 2) & 0x1C)    // uimm[4:2]
                    | ((h << 4) & 0xC0); // uimm[7:6]
            Ok(enc_i(0x03, 0x2, rd_full, 2, uimm as i32))
        }
        (0b10, 0b011) => {
            // C.FLWSP — flw rd, uimm(x2)
            let uimm = ((h >> 7) & 0x20) | ((h >> 2) & 0x1C) | ((h << 4) & 0xC0);
            Ok(enc_i(0x07, 0x2, rd_full, 2, uimm as i32))
        }
        (0b10, 0b100) => {
            let bit12 = (h >> 12) & 1;
            if bit12 == 0 {
                if rs2_full == 0 {
                    // C.JR — jalr x0, 0(rs1)
                    if rd_full == 0 {
                        return Err(Trap::IllegalInstruction);
                    }
                    Ok(enc_i(0x67, 0x0, 0, rd_full, 0))
                } else {
                    // C.MV — add rd, x0, rs2  (rd=0 ⇒ HINT)
                    Ok(enc_r(0x33, 0x0, 0x00, rd_full, 0, rs2_full))
                }
            } else if rd_full == 0 && rs2_full == 0 {
                // C.EBREAK
                Ok(0x00100073)
            } else if rs2_full == 0 {
                // C.JALR — jalr x1, 0(rs1)
                Ok(enc_i(0x67, 0x0, 1, rd_full, 0))
            } else {
                // C.ADD — add rd, rd, rs2  (rd=0 ⇒ HINT)
                Ok(enc_r(0x33, 0x0, 0x00, rd_full, rd_full, rs2_full))
            }
        }
        (0b10, 0b101) => {
            // C.FSDSP — fsd rs2, uimm(x2)  (uimm[5:3|8:6])
            let uimm = ((h >> 7) & 0x38)    // uimm[5:3]
                    | ((h >> 1) & 0x1C0); // uimm[8:6]
            Ok(enc_s(0x27, 0x3, 2, rs2_full, uimm as i32))
        }
        (0b10, 0b110) => {
            // C.SWSP — sw rs2, uimm(x2)
            let uimm = ((h >> 7) & 0x3C)   // uimm[5:2]
                    | ((h >> 1) & 0xC0); // uimm[7:6]
            Ok(enc_s(0x23, 0x2, 2, rs2_full, uimm as i32))
        }
        (0b10, 0b111) => {
            // C.FSWSP — fsw rs2, uimm(x2)
            let uimm = ((h >> 7) & 0x3C) | ((h >> 1) & 0xC0);
            Ok(enc_s(0x27, 0x2, 2, rs2_full, uimm as i32))
        }

        // F/D-extension RVC slots (c.fld/c.flw/c.fsd/c.fsw, c.fldsp/...) and
        // op=0b11 (which is a 32-bit instruction, not compressed at all).
        _ => Err(Trap::IllegalInstruction),
    }
}

// --- canonical 32-bit instruction encoders, used by expand_compressed ---

fn enc_r(opcode: u32, funct3: u32, funct7: u32, rd: u32, rs1: u32, rs2: u32) -> u32 {
    (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
}

fn enc_i(opcode: u32, funct3: u32, rd: u32, rs1: u32, imm: i32) -> u32 {
    (((imm as u32) & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
}

fn enc_s(opcode: u32, funct3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let i = imm as u32;
    (((i >> 5) & 0x7F) << 25)
        | (rs2 << 20)
        | (rs1 << 15)
        | (funct3 << 12)
        | ((i & 0x1F) << 7)
        | opcode
}

fn enc_b(opcode: u32, funct3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let i = imm as u32;
    (((i >> 12) & 1) << 31)
        | (((i >> 5) & 0x3F) << 25)
        | (rs2 << 20)
        | (rs1 << 15)
        | (funct3 << 12)
        | (((i >> 1) & 0xF) << 8)
        | (((i >> 11) & 1) << 7)
        | opcode
}

fn enc_j(opcode: u32, rd: u32, imm: i32) -> u32 {
    let i = imm as u32;
    (((i >> 20) & 1) << 31)
        | (((i >> 1) & 0x3FF) << 21)
        | (((i >> 11) & 1) << 20)
        | (((i >> 12) & 0xFF) << 12)
        | (rd << 7)
        | opcode
}

fn enc_u(opcode: u32, rd: u32, imm: u32) -> u32 {
    (imm & 0xFFFFF000) | (rd << 7) | opcode
}

/// Sign-extend `value` whose sign lives at `sign_bit` (0-indexed).
fn sext(value: u32, sign_bit: u32) -> i32 {
    let shift = 31 - sign_bit;
    ((value << shift) as i32) >> shift
}

/// CJ-type 12-bit signed immediate used by c.j and c.jal.
fn compressed_j_imm(h: u32) -> i32 {
    let bits = ((h >> 1) & 0x800)   // imm[11] from inst[12]
            | ((h << 2) & 0x400)    // imm[10] from inst[8]
            | ((h >> 1) & 0x300)    // imm[9:8] from inst[10:9]
            | ((h << 1) & 0x80)     // imm[7]  from inst[6]
            | ((h >> 1) & 0x40)     // imm[6]  from inst[7]
            | ((h << 3) & 0x20)     // imm[5]  from inst[2]
            | ((h >> 7) & 0x10)     // imm[4]  from inst[11]
            | ((h >> 2) & 0xE); // imm[3:1] from inst[5:3]
    sext(bits, 11)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_addi() {
        // addi x1, x0, 42 => imm=42, rs1=0, funct3=000, rd=1, opcode=0010011
        // 0000_0010_1010_0000_0000_0000_1001_0011 = 0x02A00093
        let inst = decode(0x02A00093).unwrap();
        assert_eq!(
            inst,
            Instruction::Addi {
                rd: 1,
                rs1: 0,
                imm: 42
            }
        );
    }

    #[test]
    fn decode_add() {
        // add x3, x1, x2 => funct7=0, rs2=2, rs1=1, funct3=000, rd=3, opcode=0110011
        // 0000000_00010_00001_000_00011_0110011 = 0x002081B3
        let inst = decode(0x002081B3).unwrap();
        assert_eq!(
            inst,
            Instruction::Add {
                rd: 3,
                rs1: 1,
                rs2: 2
            }
        );
    }

    #[test]
    fn decode_lui() {
        // lui x5, 0x12345 => imm=0x12345000, rd=5, opcode=0110111
        // 0001_0010_0011_0100_0101_00101_0110111 = 0x123452B7
        let inst = decode(0x123452B7).unwrap();
        assert_eq!(
            inst,
            Instruction::Lui {
                rd: 5,
                imm: 0x12345000
            }
        );
    }

    #[test]
    fn decode_beq() {
        // beq x1, x2, +8
        // imm[12|10:5] = 0000000, rs2=2, rs1=1, funct3=000, imm[4:1|11] = 01000, opcode=1100011
        // 0000000_00010_00001_000_01000_1100011 = 0x00208463
        let inst = decode(0x00208463).unwrap();
        assert_eq!(
            inst,
            Instruction::Beq {
                rs1: 1,
                rs2: 2,
                imm: 8
            }
        );
    }

    #[test]
    fn decode_negative_imm() {
        // addi x1, x0, -1 => imm=0xFFF, rs1=0, funct3=000, rd=1, opcode=0010011
        // 1111_1111_1111_00000_000_00001_0010011 = 0xFFF00093
        let inst = decode(0xFFF00093).unwrap();
        assert_eq!(
            inst,
            Instruction::Addi {
                rd: 1,
                rs1: 0,
                imm: -1
            }
        );
    }

    #[test]
    fn imm_s_encode() {
        // sw x2, 4(x1) => imm=4, rs2=2, rs1=1, funct3=010, opcode=0100011
        // imm[11:5]=0000000, rs2=00010, rs1=00001, funct3=010, imm[4:0]=00100, op=0100011
        // 0000000_00010_00001_010_00100_0100011 = 0x0020A223
        let inst = decode(0x0020A223).unwrap();
        assert_eq!(
            inst,
            Instruction::Sw {
                rs1: 1,
                rs2: 2,
                imm: 4
            }
        );
    }

    // --- RVC expansion tests: each compressed encoding must produce the same
    // Instruction as the canonical 32-bit form decoded directly. ---

    fn expand(h: u16) -> Instruction {
        decode(expand_compressed(h).unwrap()).unwrap()
    }

    #[test]
    fn c_addi() {
        // c.addi x8, 1  => addi x8, x8, 1
        // 000 0 01000 00001 01 = 0x0405
        assert_eq!(
            expand(0x0405),
            Instruction::Addi {
                rd: 8,
                rs1: 8,
                imm: 1
            }
        );
        // c.addi x8, -1 => addi x8, x8, -1  (imm=0b11111, bit12=1)
        // 000 1 01000 11111 01 = 0x147D
        assert_eq!(
            expand(0x147D),
            Instruction::Addi {
                rd: 8,
                rs1: 8,
                imm: -1
            }
        );
    }

    #[test]
    fn c_li() {
        // c.li x5, 7 => addi x5, x0, 7
        // 010 0 00101 00111 01 = 0x429D
        assert_eq!(
            expand(0x429D),
            Instruction::Addi {
                rd: 5,
                rs1: 0,
                imm: 7
            }
        );
    }

    #[test]
    fn c_lui() {
        // c.lui x6, 1 => lui x6, 0x1000 (1 << 12)
        // 011 0 00110 00001 01 = 0x6305
        assert_eq!(expand(0x6305), Instruction::Lui { rd: 6, imm: 0x1000 });
    }

    #[test]
    fn c_addi16sp() {
        // c.addi16sp 16 => addi x2, x2, 16  (nzimm[4] = inst[6] = 1)
        // 011 0 00010 1 00 00 0 01 = 0x6141
        assert_eq!(
            expand(0x6141),
            Instruction::Addi {
                rd: 2,
                rs1: 2,
                imm: 16
            }
        );
    }

    #[test]
    fn c_addi4spn() {
        // c.addi4spn x8, 4 => addi x8, x2, 4 (nzuimm[2]=1)
        // 000 0 0000 01 000 00 = 0x0040
        assert_eq!(
            expand(0x0040),
            Instruction::Addi {
                rd: 8,
                rs1: 2,
                imm: 4
            }
        );
    }

    #[test]
    fn c_lw_sw() {
        // c.lw x8, 0(x9) => lw x8, 0(x9)
        // 010 000 001 00 000 00 = 0x4080
        assert_eq!(
            expand(0x4080),
            Instruction::Lw {
                rd: 8,
                rs1: 9,
                imm: 0
            }
        );
        // c.sw x8, 0(x9) => sw x8, 0(x9)   (rs2'=x8, rs1'=x9, uimm=0)
        // 110 000 001 00 000 00 = 0xC080
        assert_eq!(
            expand(0xC080),
            Instruction::Sw {
                rs1: 9,
                rs2: 8,
                imm: 0
            }
        );
    }

    #[test]
    fn c_lwsp_swsp() {
        // c.lwsp x5, 0  => lw x5, 0(x2)
        // 010 0 00101 00000 10 = 0x4282
        assert_eq!(
            expand(0x4282),
            Instruction::Lw {
                rd: 5,
                rs1: 2,
                imm: 0
            }
        );
        // c.swsp x5, 0 => sw x5, 0(x2)
        // 110 000000 00101 10 = 0xC016
        assert_eq!(
            expand(0xC016),
            Instruction::Sw {
                rs1: 2,
                rs2: 5,
                imm: 0
            }
        );
    }

    #[test]
    fn c_jal_j() {
        // c.jal +4 => jal x1, +4   (imm[3:1]=010, all others 0)
        // 001 0 0000 0 010 0 0 01 = 0x2011
        assert_eq!(expand(0x2011), Instruction::Jal { rd: 1, imm: 4 });
        // c.j +4 => jal x0, +4
        // 101 0 0000 0 010 0 0 01 = 0xA011
        assert_eq!(expand(0xA011), Instruction::Jal { rd: 0, imm: 4 });
    }

    #[test]
    fn c_jr_jalr_ebreak() {
        // c.jr x5 => jalr x0, 0(x5)
        // 100 0 00101 00000 10 = 0x8282
        assert_eq!(
            expand(0x8282),
            Instruction::Jalr {
                rd: 0,
                rs1: 5,
                imm: 0
            }
        );
        // c.jalr x5 => jalr x1, 0(x5)
        // 100 1 00101 00000 10 = 0x9282
        assert_eq!(
            expand(0x9282),
            Instruction::Jalr {
                rd: 1,
                rs1: 5,
                imm: 0
            }
        );
        // c.ebreak = 0x9002
        assert_eq!(expand(0x9002), Instruction::Ebreak);
    }

    #[test]
    fn c_mv_add() {
        // c.mv x5, x6 => add x5, x0, x6
        // 100 0 00101 00110 10 = 0x829A
        assert_eq!(
            expand(0x829A),
            Instruction::Add {
                rd: 5,
                rs1: 0,
                rs2: 6
            }
        );
        // c.add x5, x6 => add x5, x5, x6
        // 100 1 00101 00110 10 = 0x929A
        assert_eq!(
            expand(0x929A),
            Instruction::Add {
                rd: 5,
                rs1: 5,
                rs2: 6
            }
        );
    }

    #[test]
    fn c_beqz_bnez() {
        // c.beqz x8, +4 => beq x8, x0, +4   (imm[2] = inst[4] = 1)
        // 110 000 000 10 000 01 = 0xC011
        assert_eq!(
            expand(0xC011),
            Instruction::Beq {
                rs1: 8,
                rs2: 0,
                imm: 4
            }
        );
        // c.bnez x8, +4 => bne x8, x0, +4
        // 111 000 000 10 000 01 = 0xE011
        assert_eq!(
            expand(0xE011),
            Instruction::Bne {
                rs1: 8,
                rs2: 0,
                imm: 4
            }
        );
    }

    #[test]
    fn c_slli_srli_srai_andi() {
        // c.slli x5, 1 => slli x5, x5, 1
        // 000 0 00101 00001 10 = 0x0286
        assert_eq!(
            expand(0x0286),
            Instruction::Slli {
                rd: 5,
                rs1: 5,
                shamt: 1
            }
        );
        // c.srli x8, 1 => srli x8, x8, 1     (funct2=00)
        // 100 0 00 001 00001 01 = 0x8005
        assert_eq!(
            expand(0x8005),
            Instruction::Srli {
                rd: 8,
                rs1: 8,
                shamt: 1
            }
        );
        // c.srai x8, 1 => srai x8, x8, 1     (funct2=01)
        // 100 0 01 001 00001 01 = 0x8405
        assert_eq!(
            expand(0x8405),
            Instruction::Srai {
                rd: 8,
                rs1: 8,
                shamt: 1
            }
        );
        // c.andi x8, 1 => andi x8, x8, 1     (funct2=10)
        // 100 0 10 001 00001 01 = 0x8805
        assert_eq!(
            expand(0x8805),
            Instruction::Andi {
                rd: 8,
                rs1: 8,
                imm: 1
            }
        );
    }

    #[test]
    fn c_sub_xor_or_and() {
        // c.sub x8, x9 => sub x8, x8, x9    (funct2=11, sel=00)
        // 100 0 11 000 00 001 01 = 0x8C05
        assert_eq!(
            expand(0x8C05),
            Instruction::Sub {
                rd: 8,
                rs1: 8,
                rs2: 9
            }
        );
        // c.xor x8, x9 (sel=01) -> 100 0 11 000 01 001 01 = 0x8C25
        assert_eq!(
            expand(0x8C25),
            Instruction::Xor {
                rd: 8,
                rs1: 8,
                rs2: 9
            }
        );
        // c.or  x8, x9 (sel=10) -> 100 0 11 000 10 001 01 = 0x8C45
        assert_eq!(
            expand(0x8C45),
            Instruction::Or {
                rd: 8,
                rs1: 8,
                rs2: 9
            }
        );
        // c.and x8, x9 (sel=11) -> 100 0 11 000 11 001 01 = 0x8C65
        assert_eq!(
            expand(0x8C65),
            Instruction::And {
                rd: 8,
                rs1: 8,
                rs2: 9
            }
        );
    }

    #[test]
    fn c_illegal() {
        // c.addi4spn with imm=0 is reserved
        assert!(expand_compressed(0x0000).is_err());
        // c.lui with rd=0 is reserved
        // 011 0 00000 00001 01 = 0x6005
        assert!(expand_compressed(0x6005).is_err());
        // c.jr x0 is reserved
        // 100 0 00000 00000 10 = 0x8002
        assert!(expand_compressed(0x8002).is_err());
        // c.lwsp x0 is reserved
        // 010 0 00000 00001 10 = 0x4006
        assert!(expand_compressed(0x4006).is_err());
    }
}
