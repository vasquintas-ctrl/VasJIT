use crate::core::ir::{IrBlock, IrOp, VOperand, VReg, Width};
use std::collections::{HashMap, HashSet};

pub fn run_all(block: &mut IrBlock) {
    const_fold(block);
    dead_code_elim(block);
}

fn const_fold(block: &mut IrBlock) {
    let mut known: HashMap<u32, i64> = HashMap::new();
    for op in &mut block.ops {
        subst_op(op, &known);
        let folded = match op {
            IrOp::Add { dst, a: VOperand::Imm(x), b: VOperand::Imm(y), .. } => {
                Some((dst.0, x.wrapping_add(*y)))
            }
            IrOp::Sub { dst, a: VOperand::Imm(x), b: VOperand::Imm(y), .. } => {
                Some((dst.0, x.wrapping_sub(*y)))
            }
            IrOp::And { dst, a: VOperand::Imm(x), b: VOperand::Imm(y), .. } => {
                Some((dst.0, *x & *y))
            }
            IrOp::Or { dst, a: VOperand::Imm(x), b: VOperand::Imm(y), .. } => {
                Some((dst.0, *x | *y))
            }
            IrOp::Xor { dst, a: VOperand::Imm(x), b: VOperand::Imm(y), .. } => {
                Some((dst.0, *x ^ *y))
            }
            IrOp::Mul { dst, a: VOperand::Imm(x), b: VOperand::Imm(y), .. } => {
                Some((dst.0, x.wrapping_mul(*y)))
            }
            IrOp::Shl { dst, a: VOperand::Imm(x), b: VOperand::Imm(y), .. } => {
                Some((dst.0, x.wrapping_shl((*y as u32) & 63)))
            }
            IrOp::Shr { dst, a: VOperand::Imm(x), b: VOperand::Imm(y), .. } => {
                Some((dst.0, ((*x as u64) >> ((*y as u32) & 63)) as i64))
            }
            IrOp::Sar { dst, a: VOperand::Imm(x), b: VOperand::Imm(y), .. } => {
                Some((dst.0, x.wrapping_shr((*y as u32) & 63)))
            }
            _ => None,
        };
        if let Some((dst_id, r)) = folded {
            known.insert(dst_id, r);
            *op = IrOp::Or {
                dst: VReg(dst_id),
                a: VOperand::Imm(r),
                b: VOperand::Imm(0),
                width: Width::W64,
            };
        }
    }
}

fn subst_op(op: &mut IrOp, known: &HashMap<u32, i64>) {
    let sub = |o: &mut VOperand| {
        if let VOperand::Reg(VReg(n)) = o {
            if let Some(&v) = known.get(n) {
                *o = VOperand::Imm(v);
            }
        }
    };
    match op {
        IrOp::Add { a, b, .. }
        | IrOp::Sub { a, b, .. }
        | IrOp::And { a, b, .. }
        | IrOp::Or { a, b, .. }
        | IrOp::Xor { a, b, .. }
        | IrOp::Mul { a, b, .. }
        | IrOp::Shl { a, b, .. }
        | IrOp::Shr { a, b, .. }
        | IrOp::Sar { a, b, .. }
        | IrOp::VecAdd { a, b, .. }
        | IrOp::VecMul { a, b, .. }
        | IrOp::SetFlags { a, b, .. } => {
            sub(a);
            sub(b);
        }
        IrOp::VecFma { a, b, c, .. } => {
            sub(a);
            sub(b);
            sub(c);
        }
        IrOp::Load { addr, .. } => sub(addr),
        IrOp::Store { addr, val, .. } => {
            sub(addr);
            sub(val);
        }
        IrOp::Branch { cond: Some(c), .. } => sub(c),
        IrOp::IndirectBranch { target } | IrOp::Call { target } => sub(target),
        IrOp::Intrinsic { operands, .. } => {
            for o in operands {
                sub(o);
            }
        }
        _ => {}
    }
}

fn dead_code_elim(block: &mut IrBlock) {
    let mut used = HashSet::new();
    for op in block.ops.iter().rev() {
        let keep = match op {
            IrOp::Store { .. }
            | IrOp::SetFlags { .. }
            | IrOp::Branch { .. }
            | IrOp::IndirectBranch { .. }
            | IrOp::Call { .. }
            | IrOp::Return => true,
            IrOp::Intrinsic { effects, dst, .. } => {
                !matches!(effects, crate::core::ir::SideEffects::Pure)
                    || dst.map(|d| used.contains(&d.0)).unwrap_or(false)
            }
            other => other.dst().map(|d| used.contains(&d.0)).unwrap_or(false),
        };
        if keep {
            for o in op.operands() {
                if let VOperand::Reg(VReg(n)) = o {
                    used.insert(n);
                }
            }
            if let Some(d) = op.dst() {
                used.insert(d.0);
            }
        }
    }
    block.ops.retain(|op| match op {
        IrOp::Store { .. }
        | IrOp::SetFlags { .. }
        | IrOp::Branch { .. }
        | IrOp::IndirectBranch { .. }
        | IrOp::Call { .. }
        | IrOp::Return => true,
        IrOp::Intrinsic { effects, dst, .. } => {
            !matches!(effects, crate::core::ir::SideEffects::Pure)
                || dst.map(|d| used.contains(&d.0)).unwrap_or(true)
        }
        other => other.dst().map(|d| used.contains(&d.0)).unwrap_or(true),
    });
}
