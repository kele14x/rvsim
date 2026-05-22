use crate::cpu::Hart;
use crate::decode::Instruction;
use crate::mem::Memory;
use crate::trap::Trap;

pub fn execute(hart: &mut Hart, mem: &mut dyn Memory, inst: Instruction) -> Result<(), Trap> {
    // PC of the current instruction (we already advanced PC by 4 before calling execute)
    let pc = hart.pc.wrapping_sub(4);

    match inst {
        // R-type arithmetic
        Instruction::Add { rd, rs1, rs2 } => {
            hart.regs.set(rd, hart.regs.get(rs1).wrapping_add(hart.regs.get(rs2)));
        }
        Instruction::Sub { rd, rs1, rs2 } => {
            hart.regs.set(rd, hart.regs.get(rs1).wrapping_sub(hart.regs.get(rs2)));
        }
        Instruction::Sll { rd, rs1, rs2 } => {
            let shamt = hart.regs.get(rs2) & 0x1F;
            hart.regs.set(rd, hart.regs.get(rs1) << shamt);
        }
        Instruction::Slt { rd, rs1, rs2 } => {
            let val = if (hart.regs.get(rs1) as i32) < (hart.regs.get(rs2) as i32) {
                1
            } else {
                0
            };
            hart.regs.set(rd, val);
        }
        Instruction::Sltu { rd, rs1, rs2 } => {
            let val = if hart.regs.get(rs1) < hart.regs.get(rs2) { 1 } else { 0 };
            hart.regs.set(rd, val);
        }
        Instruction::Xor { rd, rs1, rs2 } => {
            hart.regs.set(rd, hart.regs.get(rs1) ^ hart.regs.get(rs2));
        }
        Instruction::Srl { rd, rs1, rs2 } => {
            let shamt = hart.regs.get(rs2) & 0x1F;
            hart.regs.set(rd, hart.regs.get(rs1) >> shamt);
        }
        Instruction::Sra { rd, rs1, rs2 } => {
            let shamt = hart.regs.get(rs2) & 0x1F;
            hart.regs.set(rd, ((hart.regs.get(rs1) as i32) >> shamt) as u32);
        }
        Instruction::Or { rd, rs1, rs2 } => {
            hart.regs.set(rd, hart.regs.get(rs1) | hart.regs.get(rs2));
        }
        Instruction::And { rd, rs1, rs2 } => {
            hart.regs.set(rd, hart.regs.get(rs1) & hart.regs.get(rs2));
        }

        // I-type arithmetic
        Instruction::Addi { rd, rs1, imm } => {
            hart.regs.set(rd, hart.regs.get(rs1).wrapping_add(imm as u32));
        }
        Instruction::Slti { rd, rs1, imm } => {
            let val = if (hart.regs.get(rs1) as i32) < imm { 1 } else { 0 };
            hart.regs.set(rd, val);
        }
        Instruction::Sltiu { rd, rs1, imm } => {
            // imm is sign-extended, then compared as unsigned
            let val = if hart.regs.get(rs1) < (imm as u32) { 1 } else { 0 };
            hart.regs.set(rd, val);
        }
        Instruction::Xori { rd, rs1, imm } => {
            hart.regs.set(rd, hart.regs.get(rs1) ^ (imm as u32));
        }
        Instruction::Ori { rd, rs1, imm } => {
            hart.regs.set(rd, hart.regs.get(rs1) | (imm as u32));
        }
        Instruction::Andi { rd, rs1, imm } => {
            hart.regs.set(rd, hart.regs.get(rs1) & (imm as u32));
        }
        Instruction::Slli { rd, rs1, shamt } => {
            hart.regs.set(rd, hart.regs.get(rs1) << shamt);
        }
        Instruction::Srli { rd, rs1, shamt } => {
            hart.regs.set(rd, hart.regs.get(rs1) >> shamt);
        }
        Instruction::Srai { rd, rs1, shamt } => {
            hart.regs.set(rd, ((hart.regs.get(rs1) as i32) >> shamt) as u32);
        }

        // Loads
        Instruction::Lb { rd, rs1, imm } => {
            let addr = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = mem.read8(addr)? as i8 as i32 as u32;
            hart.regs.set(rd, val);
        }
        Instruction::Lh { rd, rs1, imm } => {
            let addr = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = mem.read16(addr)? as i16 as i32 as u32;
            hart.regs.set(rd, val);
        }
        Instruction::Lw { rd, rs1, imm } => {
            let addr = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = mem.read32(addr)?;
            hart.regs.set(rd, val);
        }
        Instruction::Lbu { rd, rs1, imm } => {
            let addr = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = mem.read8(addr)? as u32;
            hart.regs.set(rd, val);
        }
        Instruction::Lhu { rd, rs1, imm } => {
            let addr = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = mem.read16(addr)? as u32;
            hart.regs.set(rd, val);
        }

        // Stores
        Instruction::Sb { rs1, rs2, imm } => {
            let addr = hart.regs.get(rs1).wrapping_add(imm as u32);
            mem.write8(addr, hart.regs.get(rs2) as u8)?;
        }
        Instruction::Sh { rs1, rs2, imm } => {
            let addr = hart.regs.get(rs1).wrapping_add(imm as u32);
            mem.write16(addr, hart.regs.get(rs2) as u16)?;
        }
        Instruction::Sw { rs1, rs2, imm } => {
            let addr = hart.regs.get(rs1).wrapping_add(imm as u32);
            mem.write32(addr, hart.regs.get(rs2))?;
        }

        // Branches
        Instruction::Beq { rs1, rs2, imm } => {
            if hart.regs.get(rs1) == hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned);
                }
                hart.pc = target;
            }
        }
        Instruction::Bne { rs1, rs2, imm } => {
            if hart.regs.get(rs1) != hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned);
                }
                hart.pc = target;
            }
        }
        Instruction::Blt { rs1, rs2, imm } => {
            if (hart.regs.get(rs1) as i32) < (hart.regs.get(rs2) as i32) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned);
                }
                hart.pc = target;
            }
        }
        Instruction::Bge { rs1, rs2, imm } => {
            if (hart.regs.get(rs1) as i32) >= (hart.regs.get(rs2) as i32) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned);
                }
                hart.pc = target;
            }
        }
        Instruction::Bltu { rs1, rs2, imm } => {
            if hart.regs.get(rs1) < hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned);
                }
                hart.pc = target;
            }
        }
        Instruction::Bgeu { rs1, rs2, imm } => {
            if hart.regs.get(rs1) >= hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned);
                }
                hart.pc = target;
            }
        }

        // U-type
        Instruction::Lui { rd, imm } => {
            hart.regs.set(rd, imm);
        }
        Instruction::Auipc { rd, imm } => {
            hart.regs.set(rd, pc.wrapping_add(imm));
        }

        // Jumps
        Instruction::Jal { rd, imm } => {
            hart.regs.set(rd, hart.pc); // link address (already PC+4)
            let target = pc.wrapping_add(imm as u32);
            if target % 4 != 0 {
                return Err(Trap::InstructionAddressMisaligned);
            }
            hart.pc = target;
        }
        Instruction::Jalr { rd, rs1, imm } => {
            let target = (hart.regs.get(rs1).wrapping_add(imm as u32)) & !1;
            hart.regs.set(rd, hart.pc); // link address (already PC+4)
            if target % 4 != 0 {
                return Err(Trap::InstructionAddressMisaligned);
            }
            hart.pc = target;
        }

        // System
        Instruction::Ecall => {
            return Err(Trap::EnvironmentCallFromMMode);
        }
        Instruction::Ebreak => {
            return Err(Trap::Breakpoint);
        }
        Instruction::Mret => {
            let mepc = hart.csrs.read(crate::csr::CSR_MEPC, hart.cycle, hart.instret).unwrap_or(0);
            hart.pc = mepc;
        }
        Instruction::Wfi => {
            // No-op — in a simple simulator, just continue
        }
        Instruction::Fence => {
            // No-op in single-hart, in-order simulator
        }

        // CSR instructions
        Instruction::Csrrw { rd, rs1, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret)?;
            hart.csrs.write(csr, hart.regs.get(rs1))?;
            hart.regs.set(rd, old);
        }
        Instruction::Csrrs { rd, rs1, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret)?;
            if rs1 != 0 {
                hart.csrs.write(csr, old | hart.regs.get(rs1))?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrc { rd, rs1, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret)?;
            if rs1 != 0 {
                hart.csrs.write(csr, old & !hart.regs.get(rs1))?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrwi { rd, uimm, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret)?;
            hart.csrs.write(csr, uimm as u32)?;
            hart.regs.set(rd, old);
        }
        Instruction::Csrrsi { rd, uimm, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret)?;
            if uimm != 0 {
                hart.csrs.write(csr, old | (uimm as u32))?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrci { rd, uimm, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret)?;
            if uimm != 0 {
                hart.csrs.write(csr, old & !(uimm as u32))?;
            }
            hart.regs.set(rd, old);
        }
    }

    Ok(())
}
