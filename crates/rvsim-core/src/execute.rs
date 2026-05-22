use crate::cpu::{Hart, PRIV_U, PRIV_S};
use crate::csr::CSR_MSTATUS;
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

        // A extension
        Instruction::LrW { rd, rs1 } => {
            let addr = hart.regs.get(rs1);
            let val = mem.read32(addr)?;
            hart.regs.set(rd, val);
            hart.reservation = Some(addr);
        }
        Instruction::ScW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            if hart.reservation == Some(addr) {
                mem.write32(addr, hart.regs.get(rs2))?;
                hart.regs.set(rd, 0); // success
            } else {
                hart.regs.set(rd, 1); // failure
            }
            hart.reservation = None;
        }
        Instruction::AmoswapW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            let old = mem.read32(addr)?;
            mem.write32(addr, hart.regs.get(rs2))?;
            hart.regs.set(rd, old);
        }
        Instruction::AmoaddW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            let old = mem.read32(addr)?;
            mem.write32(addr, old.wrapping_add(hart.regs.get(rs2)))?;
            hart.regs.set(rd, old);
        }
        Instruction::AmoxorW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            let old = mem.read32(addr)?;
            mem.write32(addr, old ^ hart.regs.get(rs2))?;
            hart.regs.set(rd, old);
        }
        Instruction::AmoandW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            let old = mem.read32(addr)?;
            mem.write32(addr, old & hart.regs.get(rs2))?;
            hart.regs.set(rd, old);
        }
        Instruction::AmoorW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            let old = mem.read32(addr)?;
            mem.write32(addr, old | hart.regs.get(rs2))?;
            hart.regs.set(rd, old);
        }
        Instruction::AmominW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            let old = mem.read32(addr)?;
            let val = hart.regs.get(rs2);
            let result = if (old as i32) < (val as i32) { old } else { val };
            mem.write32(addr, result)?;
            hart.regs.set(rd, old);
        }
        Instruction::AmomaxW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            let old = mem.read32(addr)?;
            let val = hart.regs.get(rs2);
            let result = if (old as i32) > (val as i32) { old } else { val };
            mem.write32(addr, result)?;
            hart.regs.set(rd, old);
        }
        Instruction::AmominuW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            let old = mem.read32(addr)?;
            let val = hart.regs.get(rs2);
            let result = if old < val { old } else { val };
            mem.write32(addr, result)?;
            hart.regs.set(rd, old);
        }
        Instruction::AmomaxuW { rd, rs1, rs2 } => {
            let addr = hart.regs.get(rs1);
            let old = mem.read32(addr)?;
            let val = hart.regs.get(rs2);
            let result = if old > val { old } else { val };
            mem.write32(addr, result)?;
            hart.regs.set(rd, old);
        }

        // M extension
        Instruction::Mul { rd, rs1, rs2 } => {
            hart.regs.set(rd, hart.regs.get(rs1).wrapping_mul(hart.regs.get(rs2)));
        }
        Instruction::Mulh { rd, rs1, rs2 } => {
            let a = hart.regs.get(rs1) as i32 as i64;
            let b = hart.regs.get(rs2) as i32 as i64;
            hart.regs.set(rd, ((a * b) >> 32) as u32);
        }
        Instruction::Mulhsu { rd, rs1, rs2 } => {
            let a = hart.regs.get(rs1) as i32 as i64;
            let b = hart.regs.get(rs2) as u64 as i64;
            hart.regs.set(rd, ((a * b) >> 32) as u32);
        }
        Instruction::Mulhu { rd, rs1, rs2 } => {
            let a = hart.regs.get(rs1) as u64;
            let b = hart.regs.get(rs2) as u64;
            hart.regs.set(rd, ((a * b) >> 32) as u32);
        }
        Instruction::Div { rd, rs1, rs2 } => {
            let a = hart.regs.get(rs1) as i32;
            let b = hart.regs.get(rs2) as i32;
            let result = if b == 0 {
                -1i32 as u32
            } else if a == i32::MIN && b == -1 {
                a as u32
            } else {
                (a / b) as u32
            };
            hart.regs.set(rd, result);
        }
        Instruction::Divu { rd, rs1, rs2 } => {
            let a = hart.regs.get(rs1);
            let b = hart.regs.get(rs2);
            let result = if b == 0 { u32::MAX } else { a / b };
            hart.regs.set(rd, result);
        }
        Instruction::Rem { rd, rs1, rs2 } => {
            let a = hart.regs.get(rs1) as i32;
            let b = hart.regs.get(rs2) as i32;
            let result = if b == 0 {
                a as u32
            } else if a == i32::MIN && b == -1 {
                0u32
            } else {
                (a % b) as u32
            };
            hart.regs.set(rd, result);
        }
        Instruction::Remu { rd, rs1, rs2 } => {
            let a = hart.regs.get(rs1);
            let b = hart.regs.get(rs2);
            let result = if b == 0 { a } else { a % b };
            hart.regs.set(rd, result);
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
            let target = pc.wrapping_add(imm as u32);
            if target % 4 != 0 {
                return Err(Trap::InstructionAddressMisaligned);
            }
            hart.regs.set(rd, hart.pc); // link address (already PC+4)
            hart.pc = target;
        }
        Instruction::Jalr { rd, rs1, imm } => {
            let target = (hart.regs.get(rs1).wrapping_add(imm as u32)) & !1;
            if target % 4 != 0 {
                return Err(Trap::InstructionAddressMisaligned);
            }
            hart.regs.set(rd, hart.pc); // link address (already PC+4)
            hart.pc = target;
        }

        // System
        Instruction::Ecall => {
            return Err(match hart.priv_mode {
                PRIV_U => Trap::EnvironmentCallFromUMode,
                PRIV_S => Trap::EnvironmentCallFromSMode,
                _ => Trap::EnvironmentCallFromMMode,
            });
        }
        Instruction::Ebreak => {
            return Err(Trap::Breakpoint);
        }
        Instruction::Mret => {
            // Restore PC from mepc
            let mepc = hart.csrs.read_raw(crate::csr::CSR_MEPC);
            hart.pc = mepc;

            // Restore privilege and interrupt state from mstatus
            let mut mstatus = hart.csrs.read_raw(CSR_MSTATUS);
            // MPP (bits 12:11) → new privilege
            let mpp = ((mstatus >> 11) & 0x3) as u8;
            hart.priv_mode = mpp;
            // MIE = MPIE (bit 3 = bit 7)
            let mpie = (mstatus >> 7) & 1;
            mstatus = (mstatus & !(1 << 3)) | (mpie << 3);
            // MPIE = 1
            mstatus |= 1 << 7;
            // MPP = U (0)
            mstatus &= !(0x3 << 11);
            hart.csrs.write_raw(CSR_MSTATUS, mstatus);
        }
        Instruction::Sret => {
            // Restore PC from sepc
            let sepc = hart.csrs.read_raw(crate::csr::CSR_SEPC);
            hart.pc = sepc;

            // Restore privilege and interrupt state from mstatus
            let mut mstatus = hart.csrs.read_raw(CSR_MSTATUS);
            // SPP (bit 8) → new privilege (0=U, 1=S)
            let spp = ((mstatus >> 8) & 1) as u8;
            hart.priv_mode = spp;
            // SIE = SPIE (bit 1 = bit 5)
            let spie = (mstatus >> 5) & 1;
            mstatus = (mstatus & !(1 << 1)) | (spie << 1);
            // SPIE = 1
            mstatus |= 1 << 5;
            // SPP = U (0)
            mstatus &= !(1 << 8);
            hart.csrs.write_raw(CSR_MSTATUS, mstatus);
        }
        Instruction::SfenceVma => {
            // No-op until MMU is implemented
        }
        Instruction::Wfi => {
            // No-op — in a simple simulator, just continue
        }
        Instruction::Fence => {
            // No-op in single-hart, in-order simulator
        }

        // CSR instructions
        Instruction::Csrrw { rd, rs1, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            hart.csrs.write(csr, hart.regs.get(rs1), hart.priv_mode)?;
            hart.regs.set(rd, old);
        }
        Instruction::Csrrs { rd, rs1, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            if rs1 != 0 {
                hart.csrs.write(csr, old | hart.regs.get(rs1), hart.priv_mode)?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrc { rd, rs1, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            if rs1 != 0 {
                hart.csrs.write(csr, old & !hart.regs.get(rs1), hart.priv_mode)?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrwi { rd, uimm, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            hart.csrs.write(csr, uimm as u32, hart.priv_mode)?;
            hart.regs.set(rd, old);
        }
        Instruction::Csrrsi { rd, uimm, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            if uimm != 0 {
                hart.csrs.write(csr, old | (uimm as u32), hart.priv_mode)?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrci { rd, uimm, csr } => {
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            if uimm != 0 {
                hart.csrs.write(csr, old & !(uimm as u32), hart.priv_mode)?;
            }
            hart.regs.set(rd, old);
        }
    }

    Ok(())
}
