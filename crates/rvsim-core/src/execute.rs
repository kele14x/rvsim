use crate::cpu::{Hart, COUNTER_WRITTEN_CYCLE, COUNTER_WRITTEN_INSTRET, PRIV_M, PRIV_U, PRIV_S};
use crate::csr::{
    CSR_FCSR, CSR_MCYCLE, CSR_MCYCLEH, CSR_MEPC, CSR_MINSTRET, CSR_MINSTRETH, CSR_MSTATUS,
    CSR_SATP, CSR_SEPC,
    MSTATUS_SIE_BIT, MSTATUS_MIE_BIT, MSTATUS_SPIE_BIT, MSTATUS_MPIE_BIT,
    MSTATUS_SPP_BIT, MSTATUS_MPP_SHIFT, MSTATUS_MPP_MASK,
    MSTATUS_TSR, MSTATUS_TVM, MSTATUS_TW,
    MSTATUS_FS_MASK, MSTATUS_FS_DIRTY,
};
use crate::decode::Instruction;
use crate::mem::Memory;
use crate::mmu::AccessType;
use crate::trap::{Trap, TrapInfo};

/// Execute one already-decoded instruction. `pc` is the address of this
/// instruction (i.e. the value of PC before step() advanced it by the
/// instruction width).
pub fn execute(
    hart: &mut Hart,
    mem: &mut dyn Memory,
    inst: Instruction,
    pc: u32,
) -> Result<(), TrapInfo> {

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
        // LR/SC/AMO require natural alignment (4 bytes for .W).
        Instruction::LrW { rd, rs1 } => {
            let va = hart.regs.get(rs1);
            if va & 0x3 != 0 {
                return Err(TrapInfo::new(Trap::LoadAddressMisaligned, va));
            }
            let val = hart.load32(mem, va)?;
            hart.regs.set(rd, val);
            hart.reservation = Some(va);
        }
        Instruction::ScW { rd, rs1, rs2 } => {
            let va = hart.regs.get(rs1);
            if va & 0x3 != 0 {
                return Err(TrapInfo::new(Trap::StoreAddressMisaligned, va));
            }
            // SC unconditionally clears the reservation, even when the store
            // itself faults — do it before the fallible store.
            let matched = hart.reservation == Some(va);
            hart.reservation = None;
            if matched {
                hart.store32(mem, va, hart.regs.get(rs2))?;
                hart.regs.set(rd, 0); // success
            } else {
                hart.regs.set(rd, 1); // failure
            }
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
            let result = a.checked_div(b).unwrap_or(u32::MAX);
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
                if target & 1 != 0 {
                    return Err(TrapInfo::new(Trap::InstructionAddressMisaligned, target));
                }
                hart.pc = target;
            }
        }
        Instruction::Bne { rs1, rs2, imm } => {
            if hart.regs.get(rs1) != hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target & 1 != 0 {
                    return Err(TrapInfo::new(Trap::InstructionAddressMisaligned, target));
                }
                hart.pc = target;
            }
        }
        Instruction::Blt { rs1, rs2, imm } => {
            if (hart.regs.get(rs1) as i32) < (hart.regs.get(rs2) as i32) {
                let target = pc.wrapping_add(imm as u32);
                if target & 1 != 0 {
                    return Err(TrapInfo::new(Trap::InstructionAddressMisaligned, target));
                }
                hart.pc = target;
            }
        }
        Instruction::Bge { rs1, rs2, imm } => {
            if (hart.regs.get(rs1) as i32) >= (hart.regs.get(rs2) as i32) {
                let target = pc.wrapping_add(imm as u32);
                if target & 1 != 0 {
                    return Err(TrapInfo::new(Trap::InstructionAddressMisaligned, target));
                }
                hart.pc = target;
            }
        }
        Instruction::Bltu { rs1, rs2, imm } => {
            if hart.regs.get(rs1) < hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target & 1 != 0 {
                    return Err(TrapInfo::new(Trap::InstructionAddressMisaligned, target));
                }
                hart.pc = target;
            }
        }
        Instruction::Bgeu { rs1, rs2, imm } => {
            if hart.regs.get(rs1) >= hart.regs.get(rs2) {
                let target = pc.wrapping_add(imm as u32);
                if target & 1 != 0 {
                    return Err(TrapInfo::new(Trap::InstructionAddressMisaligned, target));
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
            if target & 1 != 0 {
                return Err(TrapInfo::new(Trap::InstructionAddressMisaligned, target));
            }
            hart.regs.set(rd, hart.pc); // link address (PC + instruction width)
            hart.pc = target;
        }
        Instruction::Jalr { rd, rs1, imm } => {
            let target = (hart.regs.get(rs1).wrapping_add(imm as u32)) & !1;
            if target & 1 != 0 {
                return Err(TrapInfo::new(Trap::InstructionAddressMisaligned, target));
            }
            hart.regs.set(rd, hart.pc); // link address (PC + instruction width)
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

        // F extension — loads/stores
        Instruction::Flw { rd, rs1, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = hart.load32(mem, va)?;
            hart.fregs.set_f32(rd, val);
            mark_fs_dirty(hart);
        }
        Instruction::Fsw { rs1, rs2, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            hart.store32(mem, va, hart.fregs.get(rs2) as u32)?;
        }

        // F extension — arithmetic
        Instruction::FaddS { rd, rs1, rs2, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let (r, flags) = f32_add(a, b);
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FsubS { rd, rs1, rs2, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let (r, flags) = f32_sub(a, b);
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FmulS { rd, rs1, rs2, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let (r, flags) = f32_mul(a, b);
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FdivS { rd, rs1, rs2, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let (r, flags) = f32_div(a, b);
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FsqrtS { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let (r, flags) = f32_sqrt(a);
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }

        // F extension — fused multiply-add
        Instruction::FmaddS { rd, rs1, rs2, rs3, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let c = f32::from_bits(hart.fregs.get_f32(rs3));
            let (r, flags) = f32_fma(a, b, c);
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FmsubS { rd, rs1, rs2, rs3, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let c = f32::from_bits(hart.fregs.get_f32(rs3));
            let (r, flags) = f32_fma(a, b, -c);
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FnmsubS { rd, rs1, rs2, rs3, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let c = f32::from_bits(hart.fregs.get_f32(rs3));
            let (r, flags) = f32_fma(-a, b, c);
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FnmaddS { rd, rs1, rs2, rs3, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let c = f32::from_bits(hart.fregs.get_f32(rs3));
            let (r, flags) = f32_fma(-a, b, -c);
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }

        // F extension — sign injection
        Instruction::FsgnjS { rd, rs1, rs2 } => {
            let a = hart.fregs.get_f32(rs1);
            let b = hart.fregs.get_f32(rs2);
            hart.fregs.set_f32(rd, (a & 0x7FFF_FFFF) | (b & 0x8000_0000));
            mark_fs_dirty(hart);
        }
        Instruction::FsgnjnS { rd, rs1, rs2 } => {
            let a = hart.fregs.get_f32(rs1);
            let b = hart.fregs.get_f32(rs2);
            hart.fregs.set_f32(rd, (a & 0x7FFF_FFFF) | (!b & 0x8000_0000));
            mark_fs_dirty(hart);
        }
        Instruction::FsgnjxS { rd, rs1, rs2 } => {
            let a = hart.fregs.get_f32(rs1);
            let b = hart.fregs.get_f32(rs2);
            hart.fregs.set_f32(rd, a ^ (b & 0x8000_0000));
            mark_fs_dirty(hart);
        }

        // F extension — min/max
        Instruction::FminS { rd, rs1, rs2 } => {
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let mut flags = 0u32;
            if is_snan_f32(a) || is_snan_f32(b) { flags |= NV; }
            let r = if a.is_nan() && b.is_nan() {
                f32::from_bits(0x7FC0_0000) // canonical NaN
            } else if a.is_nan() {
                b
            } else if b.is_nan() {
                a
            } else if a == 0.0 && b == 0.0 {
                // -0 < +0
                if a.is_sign_negative() { a } else { b }
            } else if a < b { a } else { b };
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FmaxS { rd, rs1, rs2 } => {
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let mut flags = 0u32;
            if is_snan_f32(a) || is_snan_f32(b) { flags |= NV; }
            let r = if a.is_nan() && b.is_nan() {
                f32::from_bits(0x7FC0_0000)
            } else if a.is_nan() {
                b
            } else if b.is_nan() {
                a
            } else if a == 0.0 && b == 0.0 {
                if a.is_sign_positive() { a } else { b }
            } else if a > b { a } else { b };
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }

        // F extension — compare (result to integer rd)
        Instruction::FeqS { rd, rs1, rs2 } => {
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let mut flags = 0u32;
            if is_snan_f32(a) || is_snan_f32(b) { flags |= NV; }
            let result = if a.is_nan() || b.is_nan() { 0u32 } else if a == b { 1 } else { 0 };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }
        Instruction::FltS { rd, rs1, rs2 } => {
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let mut flags = 0u32;
            if a.is_nan() || b.is_nan() { flags |= NV; }
            let result = if a.is_nan() || b.is_nan() { 0u32 } else if a < b { 1 } else { 0 };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }
        Instruction::FleS { rd, rs1, rs2 } => {
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let b = f32::from_bits(hart.fregs.get_f32(rs2));
            let mut flags = 0u32;
            if a.is_nan() || b.is_nan() { flags |= NV; }
            let result = if a.is_nan() || b.is_nan() { 0u32 } else if a <= b { 1 } else { 0 };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }

        // F extension — classify
        Instruction::FclassS { rd, rs1 } => {
            let v = f32::from_bits(hart.fregs.get_f32(rs1));
            hart.regs.set(rd, classify_f32(v));
        }

        // F extension — convert float → int
        Instruction::FcvtWS { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let mut flags = 0u32;
            let result = if a.is_nan() || a >= 2147483648.0f32 {
                flags |= NV;
                i32::MAX as u32
            } else if a < -2147483648.0f32 {
                flags |= NV;
                i32::MIN as u32
            } else {
                let i = a as i32;
                if i as f32 != a { flags |= NX; }
                i as u32
            };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }
        Instruction::FcvtWuS { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let a = f32::from_bits(hart.fregs.get_f32(rs1));
            let mut flags = 0u32;
            let result = if a.is_nan() || a >= 4294967296.0f32 {
                flags |= NV;
                u32::MAX
            } else if a <= -1.0f32 {
                flags |= NV;
                0u32
            } else if a < 0.0 && a > -1.0 {
                flags |= NX;
                0u32
            } else {
                let u = a as u32;
                if u as f32 != a { flags |= NX; }
                u
            };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }

        // F extension — convert int → float
        Instruction::FcvtSW { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let i = hart.regs.get(rs1) as i32;
            let r = i as f32;
            let mut flags = 0u32;
            if r as i32 != i { flags |= NX; }
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FcvtSWu { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let u = hart.regs.get(rs1);
            let r = u as f32;
            let mut flags = 0u32;
            if r as u32 != u { flags |= NX; }
            hart.fregs.set_f32(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }

        // F extension — move/reinterpret
        Instruction::FmvXW { rd, rs1 } => {
            hart.regs.set(rd, hart.fregs.get(rs1) as u32);
        }
        Instruction::FmvWX { rd, rs1 } => {
            hart.fregs.set_f32(rd, hart.regs.get(rs1));
            mark_fs_dirty(hart);
        }

        // F/D — convert between S and D
        Instruction::FcvtSD { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let d = f64::from_bits(hart.fregs.get(rs1));
            let mut flags = 0u32;
            let rbits = if d.is_nan() {
                if is_snan_f64(d) { flags |= NV; }
                0x7FC0_0000u32 // canonical f32 qNaN
            } else {
                let r = d as f32;
                flags |= inexact_flags(r, d);
                r.to_bits()
            };
            hart.fregs.set_f32(rd, rbits);
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FcvtDS { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let s = f32::from_bits(hart.fregs.get_f32(rs1));
            let mut flags = 0u32;
            let rbits = if s.is_nan() {
                if is_snan_f32(s) { flags |= NV; }
                0x7FF8_0000_0000_0000u64 // canonical f64 qNaN
            } else {
                (s as f64).to_bits()
            };
            hart.fregs.set(rd, rbits);
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }

        // D extension — loads/stores
        Instruction::Fld { rd, rs1, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            let val = hart.load64(mem, va)?;
            hart.fregs.set(rd, val);
            mark_fs_dirty(hart);
        }
        Instruction::Fsd { rs1, rs2, imm } => {
            let va = hart.regs.get(rs1).wrapping_add(imm as u32);
            hart.store64(mem, va, hart.fregs.get(rs2))?;
        }

        // D extension — arithmetic
        Instruction::FaddD { rd, rs1, rs2, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let (r, flags) = f64_add(a, b);
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FsubD { rd, rs1, rs2, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let (r, flags) = f64_sub(a, b);
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FmulD { rd, rs1, rs2, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let (r, flags) = f64_mul(a, b);
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FdivD { rd, rs1, rs2, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let (r, flags) = f64_div(a, b);
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FsqrtD { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let (r, flags) = f64_sqrt(a);
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }

        // D extension — fused multiply-add
        Instruction::FmaddD { rd, rs1, rs2, rs3, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let c = f64::from_bits(hart.fregs.get(rs3));
            let (r, flags) = f64_fma(a, b, c);
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FmsubD { rd, rs1, rs2, rs3, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let c = f64::from_bits(hart.fregs.get(rs3));
            let (r, flags) = f64_fma(a, b, -c);
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FnmsubD { rd, rs1, rs2, rs3, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let c = f64::from_bits(hart.fregs.get(rs3));
            let (r, flags) = f64_fma(-a, b, c);
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FnmaddD { rd, rs1, rs2, rs3, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let c = f64::from_bits(hart.fregs.get(rs3));
            let (r, flags) = f64_fma(-a, b, -c);
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }

        // D extension — sign injection
        Instruction::FsgnjD { rd, rs1, rs2 } => {
            let a = hart.fregs.get(rs1);
            let b = hart.fregs.get(rs2);
            hart.fregs.set(rd, (a & 0x7FFF_FFFF_FFFF_FFFF) | (b & 0x8000_0000_0000_0000));
            mark_fs_dirty(hart);
        }
        Instruction::FsgnjnD { rd, rs1, rs2 } => {
            let a = hart.fregs.get(rs1);
            let b = hart.fregs.get(rs2);
            hart.fregs.set(rd, (a & 0x7FFF_FFFF_FFFF_FFFF) | (!b & 0x8000_0000_0000_0000));
            mark_fs_dirty(hart);
        }
        Instruction::FsgnjxD { rd, rs1, rs2 } => {
            let a = hart.fregs.get(rs1);
            let b = hart.fregs.get(rs2);
            hart.fregs.set(rd, a ^ (b & 0x8000_0000_0000_0000));
            mark_fs_dirty(hart);
        }

        // D extension — min/max
        Instruction::FminD { rd, rs1, rs2 } => {
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let mut flags = 0u32;
            if is_snan_f64(a) || is_snan_f64(b) { flags |= NV; }
            let r = if a.is_nan() && b.is_nan() {
                f64::from_bits(0x7FF8_0000_0000_0000)
            } else if a.is_nan() {
                b
            } else if b.is_nan() {
                a
            } else if a == 0.0 && b == 0.0 {
                if a.is_sign_negative() { a } else { b }
            } else if a < b { a } else { b };
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }
        Instruction::FmaxD { rd, rs1, rs2 } => {
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let mut flags = 0u32;
            if is_snan_f64(a) || is_snan_f64(b) { flags |= NV; }
            let r = if a.is_nan() && b.is_nan() {
                f64::from_bits(0x7FF8_0000_0000_0000)
            } else if a.is_nan() {
                b
            } else if b.is_nan() {
                a
            } else if a == 0.0 && b == 0.0 {
                if a.is_sign_positive() { a } else { b }
            } else if a > b { a } else { b };
            hart.fregs.set(rd, r.to_bits());
            accrue_flags(hart, flags);
            mark_fs_dirty(hart);
        }

        // D extension — compare (result to integer rd)
        Instruction::FeqD { rd, rs1, rs2 } => {
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let mut flags = 0u32;
            if is_snan_f64(a) || is_snan_f64(b) { flags |= NV; }
            let result = if a.is_nan() || b.is_nan() { 0u32 } else if a == b { 1 } else { 0 };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }
        Instruction::FltD { rd, rs1, rs2 } => {
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let mut flags = 0u32;
            if a.is_nan() || b.is_nan() { flags |= NV; }
            let result = if a.is_nan() || b.is_nan() { 0u32 } else if a < b { 1 } else { 0 };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }
        Instruction::FleD { rd, rs1, rs2 } => {
            let a = f64::from_bits(hart.fregs.get(rs1));
            let b = f64::from_bits(hart.fregs.get(rs2));
            let mut flags = 0u32;
            if a.is_nan() || b.is_nan() { flags |= NV; }
            let result = if a.is_nan() || b.is_nan() { 0u32 } else if a <= b { 1 } else { 0 };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }

        // D extension — classify
        Instruction::FclassD { rd, rs1 } => {
            let v = f64::from_bits(hart.fregs.get(rs1));
            hart.regs.set(rd, classify_f64(v));
        }

        // D extension — convert double → int
        Instruction::FcvtWD { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let mut flags = 0u32;
            let result = if a.is_nan() || a >= 2147483648.0 {
                flags |= NV;
                i32::MAX as u32
            } else if a <= -2147483649.0 {
                flags |= NV;
                i32::MIN as u32
            } else {
                let i = a as i32;
                if i as f64 != a { flags |= NX; }
                i as u32
            };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }
        Instruction::FcvtWuD { rd, rs1, rm } => {
            resolve_rm(hart, rm)?;
            let a = f64::from_bits(hart.fregs.get(rs1));
            let mut flags = 0u32;
            let result = if a.is_nan() || a >= 4294967296.0 {
                flags |= NV;
                u32::MAX
            } else if a <= -1.0 {
                flags |= NV;
                0u32
            } else if a < 0.0 {
                flags |= NX;
                0u32
            } else {
                let u = a as u32;
                if u as f64 != a { flags |= NX; }
                u
            };
            hart.regs.set(rd, result);
            accrue_flags(hart, flags);
        }

        // D extension — convert int → double (always exact for 32-bit ints)
        Instruction::FcvtDW { rd, rs1, rm: _ } => {
            let i = hart.regs.get(rs1) as i32;
            hart.fregs.set(rd, (i as f64).to_bits());
            mark_fs_dirty(hart);
        }
        Instruction::FcvtDWu { rd, rs1, rm: _ } => {
            let u = hart.regs.get(rs1);
            hart.fregs.set(rd, (u as f64).to_bits());
            mark_fs_dirty(hart);
        }

        // CSR instructions
        Instruction::Csrrw { rd, rs1, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.mtime, hart.priv_mode)?;
            let val = hart.regs.get(rs1);
            csr_write(hart, csr, val)?;
            hart.regs.set(rd, old);
        }
        Instruction::Csrrs { rd, rs1, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.mtime, hart.priv_mode)?;
            if rs1 != 0 {
                csr_write(hart, csr, old | hart.regs.get(rs1))?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrc { rd, rs1, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.mtime, hart.priv_mode)?;
            if rs1 != 0 {
                csr_write(hart, csr, old & !hart.regs.get(rs1))?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrwi { rd, uimm, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.mtime, hart.priv_mode)?;
            csr_write(hart, csr, uimm as u32)?;
            hart.regs.set(rd, old);
        }
        Instruction::Csrrsi { rd, uimm, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.mtime, hart.priv_mode)?;
            if uimm != 0 {
                csr_write(hart, csr, old | (uimm as u32))?;
            }
            hart.regs.set(rd, old);
        }
        Instruction::Csrrci { rd, uimm, csr } => {
            tvm_check(hart, csr)?;
            let old = hart.csrs.read(csr, hart.cycle, hart.instret, hart.mtime, hart.priv_mode)?;
            if uimm != 0 {
                csr_write(hart, csr, old & !(uimm as u32))?;
            }
            hart.regs.set(rd, old);
        }
    }

    Ok(())
}

/// CSR write with mcycle/minstret alias handling. Writes to those CSRs update
/// `hart.cycle`/`hart.instret` directly and mark the counter as written so the
/// implicit post-retire bump is skipped this step.
fn csr_write(hart: &mut Hart, csr: u16, val: u32) -> Result<(), Trap> {
    hart.csrs.write(csr, val, hart.priv_mode)?;
    match csr {
        CSR_MCYCLE => {
            hart.cycle = (hart.cycle & 0xFFFF_FFFF_0000_0000) | (val as u64);
            hart.counter_written |= COUNTER_WRITTEN_CYCLE;
        }
        CSR_MCYCLEH => {
            hart.cycle = (hart.cycle & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32);
            hart.counter_written |= COUNTER_WRITTEN_CYCLE;
        }
        CSR_MINSTRET => {
            hart.instret = (hart.instret & 0xFFFF_FFFF_0000_0000) | (val as u64);
            hart.counter_written |= COUNTER_WRITTEN_INSTRET;
        }
        CSR_MINSTRETH => {
            hart.instret = (hart.instret & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32);
            hart.counter_written |= COUNTER_WRITTEN_INSTRET;
        }
        _ => {}
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

// IEEE 754 exception flag bits for fflags / fcsr.
const NV: u32 = 1 << 4; // invalid operation
const DZ: u32 = 1 << 3; // divide by zero
const OF: u32 = 1 << 2; // overflow
const UF: u32 = 1 << 1; // underflow
const NX: u32 = 1 << 0; // inexact

fn mark_fs_dirty(hart: &mut Hart) {
    let ms = hart.csrs.read_raw(CSR_MSTATUS);
    hart.csrs.write_raw(CSR_MSTATUS, (ms & !MSTATUS_FS_MASK) | MSTATUS_FS_DIRTY);
}

fn accrue_flags(hart: &mut Hart, flags: u32) {
    if flags != 0 {
        let fcsr = hart.csrs.read_raw(CSR_FCSR);
        hart.csrs.write_raw(CSR_FCSR, fcsr | flags);
    }
}

fn resolve_rm(hart: &Hart, rm: u8) -> Result<u8, TrapInfo> {
    let effective = if rm == 7 {
        (hart.csrs.read_raw(CSR_FCSR) >> 5) as u8 & 0x7
    } else {
        rm
    };
    if effective >= 5 {
        return Err(Trap::IllegalInstruction.into());
    }
    Ok(effective)
}

fn is_snan_f32(v: f32) -> bool {
    let b = v.to_bits();
    (b & 0x7F80_0000) == 0x7F80_0000
        && (b & 0x007F_FFFF) != 0
        && (b & 0x0040_0000) == 0
}

fn classify_f32(v: f32) -> u32 {
    let b = v.to_bits();
    let sign = b >> 31;
    let exp = (b >> 23) & 0xFF;
    let frac = b & 0x007F_FFFF;

    if exp == 0xFF {
        if frac == 0 {
            if sign != 0 { 1 << 0 } else { 1 << 7 } // ±inf
        } else if frac & 0x0040_0000 != 0 {
            1 << 9 // quiet NaN
        } else {
            1 << 8 // signaling NaN
        }
    } else if exp == 0 {
        if frac == 0 {
            if sign != 0 { 1 << 3 } else { 1 << 4 } // ±zero
        } else if sign != 0 {
            1 << 2 // negative subnormal
        } else {
            1 << 5 // positive subnormal
        }
    } else if sign != 0 {
        1 << 1 // negative normal
    } else {
        1 << 6 // positive normal
    }
}

fn nan_check_2(a: f32, b: f32, r: f32) -> u32 {
    if r.is_nan() {
        if is_snan_f32(a) || is_snan_f32(b) || (!a.is_nan() && !b.is_nan()) {
            NV
        } else {
            0
        }
    } else {
        0
    }
}

fn inexact_flags(r: f32, exact: f64) -> u32 {
    let mut flags = 0u32;
    if r.is_infinite() && exact.is_finite() {
        flags |= OF | NX;
    } else if r.is_finite() {
        if r as f64 != exact { flags |= NX; }
        if r.is_subnormal() && exact != 0.0 { flags |= UF; }
    }
    flags
}

fn f32_add(a: f32, b: f32) -> (f32, u32) {
    let r = a + b;
    let mut flags = nan_check_2(a, b, r);
    if flags == 0 && !r.is_nan() {
        flags |= inexact_flags(r, a as f64 + b as f64);
    }
    (r, flags)
}

fn f32_sub(a: f32, b: f32) -> (f32, u32) {
    let r = a - b;
    let mut flags = nan_check_2(a, b, r);
    if flags == 0 && !r.is_nan() {
        flags |= inexact_flags(r, a as f64 - b as f64);
    }
    (r, flags)
}

fn f32_mul(a: f32, b: f32) -> (f32, u32) {
    let r = a * b;
    let mut flags = nan_check_2(a, b, r);
    if flags == 0 && !r.is_nan() {
        flags |= inexact_flags(r, a as f64 * b as f64);
    }
    (r, flags)
}

fn f32_div(a: f32, b: f32) -> (f32, u32) {
    let r = a / b;
    let mut flags = 0u32;
    if r.is_nan() {
        if is_snan_f32(a) || is_snan_f32(b) || (!a.is_nan() && !b.is_nan()) {
            flags |= NV;
        }
    } else if b == 0.0 && !a.is_nan() {
        if a == 0.0 { flags |= NV; } else { flags |= DZ; }
    } else if !r.is_nan() {
        flags |= inexact_flags(r, a as f64 / b as f64);
    }
    (r, flags)
}

fn f32_sqrt(a: f32) -> (f32, u32) {
    let r = a.sqrt();
    let mut flags = 0u32;
    if a.is_nan() {
        if is_snan_f32(a) { flags |= NV; }
    } else if a.is_sign_negative() && a != 0.0 {
        flags |= NV;
    } else if !r.is_nan() && !r.is_infinite() {
        let exact = (a as f64).sqrt();
        flags |= inexact_flags(r, exact);
    }
    (r, flags)
}

fn f32_fma(a: f32, b: f32, c: f32) -> (f32, u32) {
    let r = a.mul_add(b, c);
    let mut flags = 0u32;
    if r.is_nan() {
        if is_snan_f32(a) || is_snan_f32(b) || is_snan_f32(c)
            || (!a.is_nan() && !b.is_nan() && !c.is_nan())
        {
            flags |= NV;
        }
    } else {
        let exact = (a as f64).mul_add(b as f64, c as f64);
        flags |= inexact_flags(r, exact);
    }
    (r, flags)
}

fn is_snan_f64(v: f64) -> bool {
    let b = v.to_bits();
    (b & 0x7FF0_0000_0000_0000) == 0x7FF0_0000_0000_0000
        && (b & 0x000F_FFFF_FFFF_FFFF) != 0
        && (b & 0x0008_0000_0000_0000) == 0
}

fn classify_f64(v: f64) -> u32 {
    let b = v.to_bits();
    let sign = b >> 63;
    let exp = (b >> 52) & 0x7FF;
    let frac = b & 0x000F_FFFF_FFFF_FFFF;

    if exp == 0x7FF {
        if frac == 0 {
            if sign != 0 { 1 << 0 } else { 1 << 7 }
        } else if frac & 0x0008_0000_0000_0000 != 0 {
            1 << 9
        } else {
            1 << 8
        }
    } else if exp == 0 {
        if frac == 0 {
            if sign != 0 { 1 << 3 } else { 1 << 4 }
        } else if sign != 0 {
            1 << 2
        } else {
            1 << 5
        }
    } else if sign != 0 {
        1 << 1
    } else {
        1 << 6
    }
}

fn nan_check_2_f64(a: f64, b: f64, r: f64) -> u32 {
    if r.is_nan() {
        if is_snan_f64(a) || is_snan_f64(b) || (!a.is_nan() && !b.is_nan()) {
            NV
        } else {
            0
        }
    } else {
        0
    }
}



fn f64_flags_finite(r: f64) -> u32 {
    if r.is_subnormal() && r != 0.0 { UF } else { 0 }
}

fn f64_add(a: f64, b: f64) -> (f64, u32) {
    let r = a + b;
    let mut flags = nan_check_2_f64(a, b, r);
    if flags == 0 && !r.is_nan() {
        if r.is_infinite() && a.is_finite() && b.is_finite() {
            flags |= OF | NX;
        } else if r.is_finite() {
            flags |= f64_flags_finite(r);
            // FMA-based exactness: a + b == r iff (a - r) + b == 0
            if (-r).mul_add(1.0, a) + b != 0.0 { flags |= NX; }
        }
    }
    (r, flags)
}

fn f64_sub(a: f64, b: f64) -> (f64, u32) {
    let r = a - b;
    let mut flags = nan_check_2_f64(a, b, r);
    if flags == 0 && !r.is_nan() {
        if r.is_infinite() && a.is_finite() && b.is_finite() {
            flags |= OF | NX;
        } else if r.is_finite() {
            flags |= f64_flags_finite(r);
            if (-r).mul_add(1.0, a) - b != 0.0 { flags |= NX; }
        }
    }
    (r, flags)
}

fn f64_mul(a: f64, b: f64) -> (f64, u32) {
    let r = a * b;
    let mut flags = nan_check_2_f64(a, b, r);
    if flags == 0 && !r.is_nan() {
        if r.is_infinite() && a.is_finite() && b.is_finite() {
            flags |= OF | NX;
        } else if r.is_finite() {
            flags |= f64_flags_finite(r);
            if a.mul_add(b, -r) != 0.0 { flags |= NX; }
        }
    }
    (r, flags)
}

fn f64_div(a: f64, b: f64) -> (f64, u32) {
    let r = a / b;
    let mut flags = 0u32;
    if r.is_nan() {
        if is_snan_f64(a) || is_snan_f64(b) || (!a.is_nan() && !b.is_nan()) {
            flags |= NV;
        }
    } else if b == 0.0 && !a.is_nan() {
        if a == 0.0 { flags |= NV; } else { flags |= DZ; }
    } else if r.is_infinite() && a.is_finite() && b.is_finite() {
        flags |= OF | NX;
    } else if r.is_finite() {
        flags |= f64_flags_finite(r);
        if r.mul_add(b, -a) != 0.0 { flags |= NX; }
    }
    (r, flags)
}

fn f64_sqrt(a: f64) -> (f64, u32) {
    let r = a.sqrt();
    let mut flags = 0u32;
    if a.is_nan() {
        if is_snan_f64(a) { flags |= NV; }
    } else if a.is_sign_negative() && a != 0.0 {
        flags |= NV;
    } else if r.is_finite() {
        flags |= f64_flags_finite(r);
        if r.mul_add(r, -a) != 0.0 { flags |= NX; }
    }
    (r, flags)
}

fn f64_fma(a: f64, b: f64, c: f64) -> (f64, u32) {
    let r = a.mul_add(b, c);
    let mut flags = 0u32;
    if r.is_nan() {
        if is_snan_f64(a) || is_snan_f64(b) || is_snan_f64(c)
            || (!a.is_nan() && !b.is_nan() && !c.is_nan())
        {
            flags |= NV;
        }
    } else if r.is_infinite() && a.is_finite() && b.is_finite() && c.is_finite() {
        flags |= OF | NX;
    } else if r.is_finite() {
        flags |= f64_flags_finite(r);
        // Residual: exact(a*b + c) - r. Compute as a*b - r + c via FMA.
        let residual = a.mul_add(b, -r) + c;
        if residual != 0.0 { flags |= NX; }
    }
    (r, flags)
}

/// AMO helper: translate twice (Load then Store), read, compute new value, write.
/// Both translations happen up front so that a D-bit fault on the store side is
/// raised before any architectural side effects. AMOs require 4-byte alignment.
fn amo_load_store<F>(
    hart: &Hart,
    mem: &mut dyn Memory,
    va: u32,
    op: F,
) -> Result<u32, TrapInfo>
where
    F: FnOnce(u32) -> u32,
{
    if va & 0x3 != 0 {
        return Err(TrapInfo::new(Trap::StoreAddressMisaligned, va));
    }
    let pa_load = hart.translate(mem, va, AccessType::Load)?;
    let pa_store = hart.translate(mem, va, AccessType::Store)?;
    let old = mem.read32(pa_load)?;
    let new = op(old);
    mem.write32(pa_store, new)?;
    Ok(old)
}
