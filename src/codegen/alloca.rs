use std::collections::HashMap;

use super::*;

pub struct RegAllocator {
    pub places: HashMap<Function, HashMap<Value, Place>>,
    pub spill_size: HashMap<Function, usize>,
    pub max_reg: HashMap<Function, usize>,
    active: Vec<(Range, Value, RegID)>,
    free_pool: Vec<(RegID, bool)>,
    free_regs: u32,
    offset: usize,
}

impl RegAllocator {
    pub fn alloca(&mut self, live_ranges: &LiveRange, free_regs: usize) {
        self.free_pool.resize(free_regs, ("s0".into_id(), true));
        for (&fid, ranges) in &live_ranges.ranges {
            self.init_pool(free_regs);
            self.active.clear();
            self.places.insert(fid, HashMap::new());
            self.max_reg.insert(fid, 0);
            self.free_regs = free_regs as u32;
            self.offset = 0;
            let mut max_reg = 0;
            for &(r, val) in ranges {
                self.expire_old_interval(r);
                if self.free_regs == 0 {
                    self.spill_at_interval(fid, r, val);
                } else {
                    self.alloca_register(fid, r, val);
                }
                let used_args = free_regs - self.free_regs as usize;
                if max_reg < used_args {
                    max_reg = used_args;
                }
            }
            self.spill_size.insert(fid, self.offset);
            self.max_reg.insert(fid, max_reg);
        }
    }

    fn init_pool(&mut self, total: usize) {
        for i in 0..total {
            self.free_pool[i] = (format!("s{}", i).into_id(), true);
        }
    }

    fn expire_old_interval(&mut self, r: Range) {
        let mut expired = Vec::new();
        for (idx, &(x, _, reg)) in self.active.iter().enumerate() {
            if x.end >= r.begin {
                break;
            }
            expired.push((idx, reg));
        }

        expired.sort_by(|a, b| b.0.cmp(&a.0));
        for &(idx, reg) in &expired {
            self.free_reg(reg);
            self.active.remove(idx);
        }
        self.sort_active();
    }

    fn spill_at_interval(&mut self, f: Function, r: Range, val: Value) {
        let (last_range, last_val, last_reg) = *self.active.last().unwrap();
        if last_range.end > r.end {
            let last = self.active.last_mut().unwrap();
            *last = (r, val, last_reg);
            let local_places = self.places.get_mut(&f).unwrap();
            local_places.insert(val, Place::Reg(last_reg));
            local_places
                .entry(last_val)
                .and_modify(|p| *p = Place::Mem(self.offset as i32));
            self.sort_active();
            self.offset += 4;
        } else {
            self.places
                .get_mut(&f)
                .unwrap()
                .insert(val, Place::Mem(self.offset as i32));
            self.offset += 4;
        }
    }

    fn alloca_register(&mut self, f: Function, r: Range, val: Value) {
        let reg = self.alloc_reg();
        self.places.entry(f).and_modify(|v| {
            v.insert(val, Place::Reg(reg));
        });
        self.active.push((r, val, reg));
        self.sort_active();
    }

    fn sort_active(&mut self) {
        self.active.sort_by(|a, b| a.0.end.cmp(&b.0.end));
    }

    fn alloc_reg(&mut self) -> RegID {
        self.free_regs -= 1;
        for (reg, free) in &mut self.free_pool {
            if *free {
                *free = false;
                return *reg;
            }
        }

        unreachable!()
    }

    fn free_reg(&mut self, reg: RegID) {
        self.free_regs += 1;
        for (r, free) in &mut self.free_pool {
            if *r == reg {
                *free = true;
                return;
            }
        }
    }

    pub fn new() -> Self {
        Self {
            places: HashMap::new(),
            spill_size: HashMap::new(),
            max_reg: HashMap::new(),
            active: Vec::new(),
            free_pool: Vec::new(),
            free_regs: 0,
            offset: 0,
        }
    }
}
