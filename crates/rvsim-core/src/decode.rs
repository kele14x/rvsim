use crate::trap::Trap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // R-type
    Add { rd: u8, rs1: u8, rs2: u8 },
    Sub { rd: u8, rs1: u8, rs2: u8 },
    Sll { rd: u8, rs1: u8, rs2: u8 },
    Slt { rd: u8, rs1: u8, rs2: u8 },
    Sltu { rd: u8, rs1: u8, rs2: u8 },
    Xor { rd: u8, rs1: u8, rs2: u8 },
    Srl { rd: u8, rs1: u8, rs2: u8 },
    Sra { rd: u8, rs1: u8, rs2: u8 },
    Or { rd: u8, rs1: u8, rs2: u8 },
    And { rd: u8, rs1: u8, rs2: u8 },

    // I-type (arithmetic)
    Addi { rd: u8, rs1: u8, imm: i32 },
    Slti { rd: u8, rs1: u8, imm: i32 },
    Sltiu { rd: u8, rs1: u8, imm: i32 },
    Xori { rd: u8, rs1: u8, imm: i32 },
    Ori { rd: u8, rs1: u8, imm: i32 },
    Andi { rd: u8, rs1: u8, imm: i32 },
    Slli { rd: u8, rs1: u8, shamt: u8 },
    Srli { rd: u8, rs1: u8, shamt: u8 },
    Srai { rd: u8, rs1: u8, shamt: u8 },

    // I-type (loads)
    Lb { rd: u8, rs1: u8, imm: i32 },
    Lh { rd: u8, rs1: u8, imm: i32 },
    Lw { rd: u8, rs1: u8, imm: i32 },
    Lbu { rd: u8, rs1: u8, imm: i32 },
    Lhu { rd: u8, rs1: u8, imm: i32 },

    // S-type (stores)
    Sb { rs1: u8, rs2: u8, imm: i32 },
    Sh { rs1: u8, rs2: u8, imm: i32 },
    Sw { rs1: u8, rs2: u8, imm: i32 },

    // B-type (branches)
    Beq { rs1: u8, rs2: u8, imm: i32 },
    Bne { rs1: u8, rs2: u8, imm: i32 },
    Blt { rs1: u8, rs2: u8, imm: i32 },
    Bge { rs1: u8, rs2: u8, imm: i32 },
    Bltu { rs1: u8, rs2: u8, imm: i32 },
    Bgeu { rs1: u8, rs2: u8, imm: i32 },

    // U-type
    Lui { rd: u8, imm: u32 },
    Auipc { rd: u8, imm: u32 },

    // J-type
    Jal { rd: u8, imm: i32 },
    Jalr { rd: u8, rs1: u8, imm: i32 },

    // A extension
    LrW { rd: u8, rs1: u8 },
    ScW { rd: u8, rs1: u8, rs2: u8 },
    AmoswapW { rd: u8, rs1: u8, rs2: u8 },
    AmoaddW { rd: u8, rs1: u8, rs2: u8 },
    AmoxorW { rd: u8, rs1: u8, rs2: u8 },
    AmoandW { rd: u8, rs1: u8, rs2: u8 },
    AmoorW { rd: u8, rs1: u8, rs2: u8 },
    AmominW { rd: u8, rs1: u8, rs2: u8 },
    AmomaxW { rd: u8, rs1: u8, rs2: u8 },
    AmominuW { rd: u8, rs1: u8, rs2: u8 },
    AmomaxuW { rd: u8, rs1: u8, rs2: u8 },

    // M extension
    Mul { rd: u8, rs1: u8, rs2: u8 },
    Mulh { rd: u8, rs1: u8, rs2: u8 },
    Mulhsu { rd: u8, rs1: u8, rs2: u8 },
    Mulhu { rd: u8, rs1: u8, rs2: u8 },
    Div { rd: u8, rs1: u8, rs2: u8 },
    Divu { rd: u8, rs1: u8, rs2: u8 },
    Rem { rd: u8, rs1: u8, rs2: u8 },
    Remu { rd: u8, rs1: u8, rs2: u8 },

    // System
    Ecall,
    Ebreak,
    Mret,
    Sret,
    Wfi,
    SfenceVma,
    Fence,
    Csrrw { rd: u8, rs1: u8, csr: u16 },
    Csrrs { rd: u8, rs1: u8, csr: u16 },
    Csrrc { rd: u8, rs1: u8, csr: u16 },
    Csrrwi { rd: u8, uimm: u8, csr: u16 },
    Csrrsi { rd: u8, uimm: u8, csr: u16 },
    Csrrci { rd: u8, uimm: u8, csr: u16 },
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

        // JALR
        0b1100111 => Ok(Instruction::Jalr {
            rd: rd(raw),
            rs1: rs1(raw),
            imm: imm_i(raw),
        }),

        // Branch
        0b1100011 => {
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let imm = imm_b(raw);
            match funct3(raw) {
                0b000 => Ok(Instruction::Beq { rs1: r1, rs2: r2, imm }),
                0b001 => Ok(Instruction::Bne { rs1: r1, rs2: r2, imm }),
                0b100 => Ok(Instruction::Blt { rs1: r1, rs2: r2, imm }),
                0b101 => Ok(Instruction::Bge { rs1: r1, rs2: r2, imm }),
                0b110 => Ok(Instruction::Bltu { rs1: r1, rs2: r2, imm }),
                0b111 => Ok(Instruction::Bgeu { rs1: r1, rs2: r2, imm }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // Load
        0b0000011 => {
            let d = rd(raw);
            let r1 = rs1(raw);
            let imm = imm_i(raw);
            match funct3(raw) {
                0b000 => Ok(Instruction::Lb { rd: d, rs1: r1, imm }),
                0b001 => Ok(Instruction::Lh { rd: d, rs1: r1, imm }),
                0b010 => Ok(Instruction::Lw { rd: d, rs1: r1, imm }),
                0b100 => Ok(Instruction::Lbu { rd: d, rs1: r1, imm }),
                0b101 => Ok(Instruction::Lhu { rd: d, rs1: r1, imm }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // Store
        0b0100011 => {
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let imm = imm_s(raw);
            match funct3(raw) {
                0b000 => Ok(Instruction::Sb { rs1: r1, rs2: r2, imm }),
                0b001 => Ok(Instruction::Sh { rs1: r1, rs2: r2, imm }),
                0b010 => Ok(Instruction::Sw { rs1: r1, rs2: r2, imm }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // OP-IMM
        0b0010011 => {
            let d = rd(raw);
            let r1 = rs1(raw);
            let imm = imm_i(raw);
            match funct3(raw) {
                0b000 => Ok(Instruction::Addi { rd: d, rs1: r1, imm }),
                0b010 => Ok(Instruction::Slti { rd: d, rs1: r1, imm }),
                0b011 => Ok(Instruction::Sltiu { rd: d, rs1: r1, imm }),
                0b100 => Ok(Instruction::Xori { rd: d, rs1: r1, imm }),
                0b110 => Ok(Instruction::Ori { rd: d, rs1: r1, imm }),
                0b111 => Ok(Instruction::Andi { rd: d, rs1: r1, imm }),
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
                        0b0000000 => Ok(Instruction::Srli { rd: d, rs1: r1, shamt }),
                        0b0100000 => Ok(Instruction::Srai { rd: d, rs1: r1, shamt }),
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
                (0b000, 0b0000000) => Ok(Instruction::Add { rd: d, rs1: r1, rs2: r2 }),
                (0b000, 0b0100000) => Ok(Instruction::Sub { rd: d, rs1: r1, rs2: r2 }),
                (0b001, 0b0000000) => Ok(Instruction::Sll { rd: d, rs1: r1, rs2: r2 }),
                (0b010, 0b0000000) => Ok(Instruction::Slt { rd: d, rs1: r1, rs2: r2 }),
                (0b011, 0b0000000) => Ok(Instruction::Sltu { rd: d, rs1: r1, rs2: r2 }),
                (0b100, 0b0000000) => Ok(Instruction::Xor { rd: d, rs1: r1, rs2: r2 }),
                (0b101, 0b0000000) => Ok(Instruction::Srl { rd: d, rs1: r1, rs2: r2 }),
                (0b101, 0b0100000) => Ok(Instruction::Sra { rd: d, rs1: r1, rs2: r2 }),
                (0b110, 0b0000000) => Ok(Instruction::Or { rd: d, rs1: r1, rs2: r2 }),
                (0b111, 0b0000000) => Ok(Instruction::And { rd: d, rs1: r1, rs2: r2 }),
                // M extension (funct7 = 0b0000001)
                (0b000, 0b0000001) => Ok(Instruction::Mul { rd: d, rs1: r1, rs2: r2 }),
                (0b001, 0b0000001) => Ok(Instruction::Mulh { rd: d, rs1: r1, rs2: r2 }),
                (0b010, 0b0000001) => Ok(Instruction::Mulhsu { rd: d, rs1: r1, rs2: r2 }),
                (0b011, 0b0000001) => Ok(Instruction::Mulhu { rd: d, rs1: r1, rs2: r2 }),
                (0b100, 0b0000001) => Ok(Instruction::Div { rd: d, rs1: r1, rs2: r2 }),
                (0b101, 0b0000001) => Ok(Instruction::Divu { rd: d, rs1: r1, rs2: r2 }),
                (0b110, 0b0000001) => Ok(Instruction::Rem { rd: d, rs1: r1, rs2: r2 }),
                (0b111, 0b0000001) => Ok(Instruction::Remu { rd: d, rs1: r1, rs2: r2 }),
                _ => Err(Trap::IllegalInstruction),
            }
        }

        // AMO (A extension)
        0b0101111 => {
            let d = rd(raw);
            let r1 = rs1(raw);
            let r2 = rs2(raw);
            let funct5 = (raw >> 27) & 0x1F;
            match funct5 {
                0b00010 => Ok(Instruction::LrW { rd: d, rs1: r1 }),
                0b00011 => Ok(Instruction::ScW { rd: d, rs1: r1, rs2: r2 }),
                0b00001 => Ok(Instruction::AmoswapW { rd: d, rs1: r1, rs2: r2 }),
                0b00000 => Ok(Instruction::AmoaddW { rd: d, rs1: r1, rs2: r2 }),
                0b00100 => Ok(Instruction::AmoxorW { rd: d, rs1: r1, rs2: r2 }),
                0b01100 => Ok(Instruction::AmoandW { rd: d, rs1: r1, rs2: r2 }),
                0b01000 => Ok(Instruction::AmoorW { rd: d, rs1: r1, rs2: r2 }),
                0b10000 => Ok(Instruction::AmominW { rd: d, rs1: r1, rs2: r2 }),
                0b10100 => Ok(Instruction::AmomaxW { rd: d, rs1: r1, rs2: r2 }),
                0b11000 => Ok(Instruction::AmominuW { rd: d, rs1: r1, rs2: r2 }),
                0b11100 => Ok(Instruction::AmomaxuW { rd: d, rs1: r1, rs2: r2 }),
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
                    0x00000073 => return Ok(Instruction::Ecall),
                    0x00100073 => return Ok(Instruction::Ebreak),
                    0x30200073 => return Ok(Instruction::Mret),
                    0x10200073 => return Ok(Instruction::Sret),
                    0x10500073 => return Ok(Instruction::Wfi),
                    _ => return Err(Trap::IllegalInstruction),
                }
            } else {
                let d = rd(raw);
                let csr = ((raw >> 20) & 0xFFF) as u16;
                match f3 {
                    0b001 => Ok(Instruction::Csrrw { rd: d, rs1: rs1(raw), csr }),
                    0b010 => Ok(Instruction::Csrrs { rd: d, rs1: rs1(raw), csr }),
                    0b011 => Ok(Instruction::Csrrc { rd: d, rs1: rs1(raw), csr }),
                    0b101 => Ok(Instruction::Csrrwi { rd: d, uimm: rs1(raw), csr }),
                    0b110 => Ok(Instruction::Csrrsi { rd: d, uimm: rs1(raw), csr }),
                    0b111 => Ok(Instruction::Csrrci { rd: d, uimm: rs1(raw), csr }),
                    _ => Err(Trap::IllegalInstruction),
                }
            }
        }

        _ => Err(Trap::IllegalInstruction),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_addi() {
        // addi x1, x0, 42 => imm=42, rs1=0, funct3=000, rd=1, opcode=0010011
        // 0000_0010_1010_0000_0000_0000_1001_0011 = 0x02A00093
        let inst = decode(0x02A00093).unwrap();
        assert_eq!(inst, Instruction::Addi { rd: 1, rs1: 0, imm: 42 });
    }

    #[test]
    fn decode_add() {
        // add x3, x1, x2 => funct7=0, rs2=2, rs1=1, funct3=000, rd=3, opcode=0110011
        // 0000000_00010_00001_000_00011_0110011 = 0x002081B3
        let inst = decode(0x002081B3).unwrap();
        assert_eq!(inst, Instruction::Add { rd: 3, rs1: 1, rs2: 2 });
    }

    #[test]
    fn decode_lui() {
        // lui x5, 0x12345 => imm=0x12345000, rd=5, opcode=0110111
        // 0001_0010_0011_0100_0101_00101_0110111 = 0x123452B7
        let inst = decode(0x123452B7).unwrap();
        assert_eq!(inst, Instruction::Lui { rd: 5, imm: 0x12345000 });
    }

    #[test]
    fn decode_beq() {
        // beq x1, x2, +8
        // imm[12|10:5] = 0000000, rs2=2, rs1=1, funct3=000, imm[4:1|11] = 01000, opcode=1100011
        // 0000000_00010_00001_000_01000_1100011 = 0x00208463
        let inst = decode(0x00208463).unwrap();
        assert_eq!(inst, Instruction::Beq { rs1: 1, rs2: 2, imm: 8 });
    }

    #[test]
    fn decode_negative_imm() {
        // addi x1, x0, -1 => imm=0xFFF, rs1=0, funct3=000, rd=1, opcode=0010011
        // 1111_1111_1111_00000_000_00001_0010011 = 0xFFF00093
        let inst = decode(0xFFF00093).unwrap();
        assert_eq!(inst, Instruction::Addi { rd: 1, rs1: 0, imm: -1 });
    }

    #[test]
    fn imm_s_encode() {
        // sw x2, 4(x1) => imm=4, rs2=2, rs1=1, funct3=010, opcode=0100011
        // imm[11:5]=0000000, rs2=00010, rs1=00001, funct3=010, imm[4:0]=00100, op=0100011
        // 0000000_00010_00001_010_00100_0100011 = 0x0020A223
        let inst = decode(0x0020A223).unwrap();
        assert_eq!(inst, Instruction::Sw { rs1: 1, rs2: 2, imm: 4 });
    }
}
