use crate::cpu::{Hart, PRIV_M, PRIV_U, PRIV_S};
use crate::csr::{
    CSR_MEPC, CSR_MSTATUS, CSR_SATP, CSR_SEPC,
    MSTATUS_SIE_BIT, MSTATUS_MIE_BIT, MSTATUS_SPIE_BIT, MSTATUS_MPIE_BIT,
    MSTATUS_SPP_BIT, MSTATUS_MPP_SHIFT, MSTATUS_MPP_MASK,
    MSTATUS_TSR, MSTATUS_TVM, MSTATUS_TW,
};
use crate::decode::Instruction;
use crate::mem::Memory;
use crate::mmu::AccessType;
use crate::trap::{Trap, TrapInfo};

pub fn execute(hart: &mut Hart, mem: &mut dyn Memory, inst: Instruction) -> Result<(), TrapInfo> {
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

        // A extension — translate before any side effects (rd write, reservation set).
        Instruction::LrW { rd, rs1 } => {
            let va = hart.regs.get(rs1);
            let val = hart.load32(mem, va)?;
            hart.regs.set(rd, val);
            hart.reservation = Some(va);
        }
        Instruction::ScW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            if hart.reservation == Some(va) {
                hart.store32(mem, va, hart.regs.get(rs2))?;
                hart.regs.set(rd, 0); // success
            } else {
                hart.regs.set(rd, 1); // failure
            }
            hart.reservation = None;
        }
        Instruction::AmoswapW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            let old = amo_load_store(hart, mem, va, |_| hart.regs.get(rs2))?;
            hart.regs.set(rd, old);
        }
        Instruction::AmoaddW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            let rs2v = hart.regs.get(rs2);
            let old = amo_load_store(hart, mem, va, |old| old.wrapping_add(rs2v))?;
            hart.regs.set(rd, old);
        }
        Instruction::AmoxorW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            let rs2v = hart.regs.get(rs2);
            let old = amo_load_store(hart, mem, va, |old| old ^ rs2v)?;
            hart.regs.set(rd, old);
        }
        Instruction::AmoandW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            let rs2v = hart.regs.get(rs2);
            let old = amo_load_store(hart, mem, va, |old| old & rs2v)?;
            hart.regs.set(rd, old);
        }
        Instruction::AmoorW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            let rs2v = hart.regs.get(rs2);
            let old = amo_load_store(hart, mem, va, |old| old | rs2v)?;
            hart.regs.set(rd, old);
        }
        Instruction::AmominW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            let rs2v = hart.regs.get(rs2);
            let old = amo_load_store(hart, mem, va, |old| {
                if (old as i32) < (rs2v as i32) { old } else { rs2v }
            })?;
            hart.regs.set(rd, old);
        }
        Instruction::AmomaxW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            let rs2v = hart.regs.get(rs2);
            let old = amo_load_store(hart, mem, va, |old| {
                if (old as i32) > (rs2v as i32) { old } else { rs2v }
            })?;
            hart.regs.set(rd, old);
        }
        Instruction::AmominuW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            let rs2v = hart.regs.get(rs2);
            let old = amo_load_store(hart, mem, va, |old| {
                if old < rs2v { old } else { rs2v }
            })?;
            hart.regs.set(rd, old);
        }
        Instruction::AmomaxuW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            let rs2v = hart.regs.get(rs2);
            let old = amo_load_store(hart, mem, va, |old| {
                if old > rs2v { old } else { rs2v }
            })?;
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

        // Loads — translate, read, then write rd (no rd update on fault).
        Instruction::Lb { rd, rs1, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = hart.load8(mem, va)? as i8 as i32 as u32;
            hart.regs.set(rd, val);
        }
        Instruction::Lh { rd, rs1, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = hart.load16(mem, va)? as i16 as i32 as u32;
            hart.regs.set(rd, val);
        }
        Instruction::Lw { rd, rs1, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = hart.load32(mem, va)?;
            hart.regs.set(rd, val);
        }
        Instruction::Lbu { rd, rs1, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = hart.load8(mem, va)? as u32;
            hart.regs.set(rd, val);
        }
        Instruction::Lhu { rd, rs1, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = hart.load16(mem, va)? as u32;
            hart.regs.set(rd, val);
        }

        // Stores
        Instruction::Sb { rs1, rs2, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            hart.store8(mem, va, hart.regs.get(rs2) as u8)?;
        }
        Instruction::Sh { rs1, rs2, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            hart.store16(mem, va, hart.regs.get(rs2) as u16)?;
        }
        Instruction::Sw { rs1, rs2, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            hart.store32(mem, va, hart.regs.get(rs2))?;
        }

        // Branches
        Instruction::Beq { rs1, rs2, imm } => {
            if hart.regs.get(rs1) == hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned.into());
                }
                hart.pc = target;
            }
        }
        Instruction::Bne { rs1, rs2, imm } => {
            if hart.regs.get(rs1) != hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned.into());
                }
                hart.pc = target;
            }
        }
        Instruction::Blt { rs1, rs2, imm } => {
            if (hart.regs.get(rs1) as i32) < (hart.regs.get(rs2) as i32) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned.into());
                }
                hart.pc = target;
            }
        }
        Instruction::Bge { rs1, rs2, imm } => {
            if (hart.regs.get(rs1) as i32) >= (hart.regs.get(rs2) as i32) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned.into());
                }
                hart.pc = target;
            }
        }
        Instruction::Bltu { rs1, rs2, imm } => {
            if hart.regs.get(rs1) < hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned.into());
                }
                hart.pc = target;
            }
        }
        Instruction::Bgeu { rs1, rs2, imm } => {
            if hart.regs.get(rs1) >= hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target % 4 != 0 {
                    return Err(Trap::InstructionAddressMisaligned.into());
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
                return Err(Trap::InstructionAddressMisaligned.into());
            }
            hart.regs.set(rd, hart.pc); // link address (already PC+4)
            hart.pc = target;
        }
        Instruction::Jalr { rd, rs1, imm } => {
            let target = (hart.regs.get(rs1).wrapping_add(imm as u32)) & !1;
            if target % 4 != 0 {
                return Err(Trap::InstructionAddressMisaligned.into());
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
            }.into());
        }
        Instruction::Ebreak => {
            return Err(Trap::Breakpoint.into());
        }
        Instruction::Mret => {
            if hart.priv_mode < PRIV_M {
                return Err(Trap::IllegalInstruction.into());
            }
            hart.pc = hart.csrs.read_raw(CSR_MEPC);
            hart.priv_mode = hart.csrs.mstatus_trap_return(
                MSTATUS_MIE_BIT, MSTATUS_MPIE_BIT,
                MSTATUS_MPP_SHIFT, MSTATUS_MPP_MASK,
            );
        }
        Instruction::Sret => {
            // SRET requires at least S; mstatus.TSR=1 traps SRET in S-mode.
            let mstatus = hart.csrs.read_raw(CSR_MSTATUS);
            if hart.priv_mode < PRIV_S
                || (hart.priv_mode == PRIV_S && (mstatus & MSTATUS_TSR) != 0)
            {
                return Err(Trap::IllegalInstruction.into());
            }
            hart.pc = hart.csrs.read_raw(CSR_SEPC);
            hart.priv_mode = hart.csrs.mstatus_trap_return(
                MSTATUS_SIE_BIT, MSTATUS_SPIE_BIT,
                MSTATUS_SPP_BIT, 1 << MSTATUS_SPP_BIT,
            );
        }
        Instruction::SfenceVma => {
            // U-mode always traps; S-mode traps when mstatus.TVM=1.
            let mstatus = hart.csrs.read_raw(CSR_MSTATUS);
            if hart.priv_mode == PRIV_U
                || (hart.priv_mode == PRIV_S && (mstatus & MSTATUS_TVM) != 0)
            {
                return Err(Trap::IllegalInstruction.into());
            }
            // No TLB to flush.
        }
        Instruction::Wfi => {
            // mstatus.TW=1 traps WFI when executed below M-mode.
            let mstatus = hart.csrs.read_raw(CSR_MSTATUS);
            if hart.priv_mode < PRIV_M && (mstatus & MSTATUS_TW) != 0 {
                return Err(Trap::IllegalInstruction.into());
            }
            // No-op — in a simple simulator, just continue
        }
        Instruction::Fence => {
            // No-op in single-hart, in-order simulator
        }

        // CSR instructions
        Instruction::Csrrw { rd, rs1, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            hart.csrs.write(csr, hart.regs.get(rs1), hart.priv_mode)?;
            hart.regs.set(rd, old);
        }
        Instruction::Csrrs { rd, rs1, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            if rs1 != 0 {
                hart.csrs.write(csr, old | hart.regs.get(rs1), hart.priv_mode)?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrc { rd, rs1, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            if rs1 != 0 {
                hart.csrs.write(csr, old & !hart.regs.get(rs1), hart.priv_mode)?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrwi { rd, uimm, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            hart.csrs.write(csr, uimm as u32, hart.priv_mode)?;
            hart.regs.set(rd, old);
        }
        Instruction::Csrrsi { rd, uimm, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            if uimm != 0 {
                hart.csrs.write(csr, old | (uimm as u32), hart.priv_mode)?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrci { rd, uimm, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.priv_mode)?;
            if uimm != 0 {
                hart.csrs.write(csr, old & !(uimm as u32), hart.priv_mode)?;
            }
            hart.regs.set(rd, old);
        }
    }

    Ok(())
}

/// mstatus.TVM=1 traps S-mode accesses to satp as IllegalInstruction.
fn tvm_check(hart: &Hart, csr: u16) -> Result<(), TrapInfo> {
    if csr == CSR_SATP && hart.priv_mode == PRIV_S {
        let mstatus = hart.csrs.read_raw(CSR_MSTATUS);
        if mstatus & MSTATUS_TVM != 0 {
            return Err(Trap::IllegalInstruction.into());
        }
    }
    Ok(())
}

/// AMO helper: translate twice (Load then Store), read, compute new value, write.
/// Both translations happen up front so that a D-bit fault on the store side is
/// raised before any architectural side effects.
fn amo_load_store<F>(
    hart: &Hart,
    mem: &mut dyn Memory,
    va: u32,
    op: F,
) -> Result<u32, TrapInfo>
where
    F: FnOnce(u32) -> u32,
{
    let pa_load = hart.translate(mem, va, AccessType::Load)?;
    let pa_store = hart.translate(mem, va, AccessType::Store)?;
    let old = mem.read32(pa_load)?;
    let new = op(old);
    mem.write32(pa_store, new)?;
    Ok(old)
}
