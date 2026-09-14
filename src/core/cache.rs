use crate::core::jit_ffi::CompiledBlock;
use std::collections::{HashMap, VecDeque};

pub struct CodeCache {
    blocks: HashMap<u64, CachedBlock>,
    lru: VecDeque<u64>,
    max_bytes: usize,
    used_bytes: usize,
}
struct CachedBlock { block: CompiledBlock, guest_end: u64 }

impl CodeCache {
    pub fn new(max_bytes:usize)->Self {
        Self{blocks:HashMap::new(),lru:VecDeque::new(),max_bytes,used_bytes:0}
    }
    pub fn lookup(&self, pc:u64)->Option<&CompiledBlock> {
        self.blocks.get(&pc).map(|c|&c.block)
    }
    pub fn touch(&mut self, pc:u64) {
        if let Some(pos)=self.lru.iter().position(|&p|p==pc) {
            self.lru.remove(pos); self.lru.push_back(pc);
        }
    }
    pub fn insert(&mut self, guest_pc:u64, guest_end:u64, block:CompiledBlock) {
        let size=block.code_len;
        while self.used_bytes+size > self.max_bytes && !self.lru.is_empty() {
            if let Some(old)=self.lru.pop_front() {
                if let Some(ev)=self.blocks.remove(&old) {
                    self.used_bytes=self.used_bytes.saturating_sub(ev.block.code_len);
                }
            }
        }
        self.used_bytes+=size;
        self.lru.push_back(guest_pc);
        self.blocks.insert(guest_pc, CachedBlock{block,guest_end});
    }
}
