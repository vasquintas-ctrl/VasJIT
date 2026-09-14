use crate::core::ir::{IrBlock, VOperand, VReg};
use crate::ffi::{CHostGpr, CLocation, CLocationKind};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Allocation { pub map: HashMap<u32, Location> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location { Reg(CHostGpr), Spill(u32) }

const HOST_GPRS: [CHostGpr;13] = [
    CHostGpr::Rax, CHostGpr::Rbx, CHostGpr::Rcx, CHostGpr::Rdx,
    CHostGpr::Rsi, CHostGpr::Rdi, CHostGpr::R8, CHostGpr::R9,
    CHostGpr::R10, CHostGpr::R11, CHostGpr::R12, CHostGpr::R13, CHostGpr::R14,
];

#[derive(Clone)]
struct Interval { vreg:u32, start:usize, end:usize }

pub fn linear_scan(block:&IrBlock)->Allocation {
    let mut first:HashMap<u32,usize>=HashMap::new();
    let mut last:HashMap<u32,usize>=HashMap::new();
    for (i,op) in block.ops.iter().enumerate() {
        if let Some(d)=op.dst(){ first.entry(d.0).or_insert(i); last.insert(d.0,i); }
        for o in op.operands(){ if let VOperand::Reg(VReg(n))=o { first.entry(n).or_insert(i); last.insert(n,i); } }
    }
    let mut intervals:Vec<Interval>=first.iter().map(|(&v,&s)| Interval{vreg:v,start:s,end:*last.get(&v).unwrap_or(&s)}).collect();
    intervals.sort_by_key(|iv| iv.start);
    let mut active:Vec<(Interval,CHostGpr)>=Vec::new();
    let mut free:Vec<CHostGpr>=HOST_GPRS.to_vec();
    let mut map:HashMap<u32,Location>=HashMap::new();
    let mut next_spill:u32=0;
    for iv in intervals {
        active.retain(|(a,gpr)| if a.end < iv.start { free.push(*gpr); false } else { true });
        if let Some(gpr)=free.pop() {
            map.insert(iv.vreg, Location::Reg(gpr)); active.push((iv,gpr));
        } else if let Some((idx,_))=active.iter().enumerate().max_by_key(|(_, (a,_))| a.end) {
            let (spilled,gpr)=active.swap_remove(idx);
            let off=next_spill; next_spill+=8;
            map.insert(spilled.vreg, Location::Spill(off));
            map.insert(iv.vreg, Location::Reg(gpr)); active.push((iv,gpr));
        } else {
            let off=next_spill; next_spill+=8; map.insert(iv.vreg, Location::Spill(off));
        }
    }
    Allocation{map}
}
impl Allocation {
    pub fn to_c_entries(&self)->Vec<crate::ffi::CAllocEntry> {
        self.map.iter().map(|(&vreg,loc)|{
            let cloc=match loc {
                Location::Reg(g)=>CLocation{kind:CLocationKind::Reg,gpr:*g,offset:0},
                Location::Spill(off)=>CLocation{kind:CLocationKind::Spill,gpr:CHostGpr::Rax,offset:*off},
            };
            crate::ffi::CAllocEntry{vreg,loc:cloc}
        }).collect()
    }
}
