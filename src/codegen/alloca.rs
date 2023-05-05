use std::collections::HashMap;

use super::*;

pub struct RegAllocator {
    pub places: HashMap<Function, HashMap<Value, Place>>,
    pub spill_size: HashMap<Function, usize>,
    pub max_reg: HashMap<Function, usize>,
    active: Vec<(Range, Value, RegID)>,
    free_pool: Vec<(RegID, bool)>,
    free_saved_regs: u32,
    free_temp_regs: u32,
    offset: usize,
}

const SAVED_REGS: u32 = 12;
const TEMP_REGS: u32 = 12;

impl RegAllocator {
    pub fn init(&mut self, fid: Function) {
        self.init_pool();
        self.active.clear();
        self.places.insert(fid, HashMap::new());
        self.max_reg.insert(fid, 0);
        self.free_saved_regs = SAVED_REGS;
        self.free_temp_regs = TEMP_REGS;
        self.offset = 0;
    }

    pub fn alloca(&mut self, live_ranges: &LiveRange) {
        self.free_pool
            .resize((SAVED_REGS + TEMP_REGS) as usize, ("s0".into_id(), true));
        for (&fid, ranges) in &live_ranges.ranges {
            self.init(fid);
            let mut max_saved_reg = 0;
            for &(r, val) in ranges {
                self.expire_old_interval(r);
                if self.free_saved_regs + self.free_temp_regs == 0 {
                    self.spill_at_interval(fid, r, val);
                } else {
                    let allow_temp =
                        !self.contains_call(r, live_ranges.function_calls.get(&fid).unwrap());
                    self.alloca_register(fid, r, val, allow_temp);
                    let used_args = SAVED_REGS - self.free_saved_regs;
                    if max_saved_reg < used_args {
                        max_saved_reg = used_args;
                    }
                }
            }
            self.spill_size.insert(fid, self.offset);
            self.max_reg.insert(fid, max_saved_reg as usize);
        }
    }

    fn init_pool(&mut self) {
        for i in 0..SAVED_REGS {
            self.free_pool[i as usize] = (format!("s{}", i).into_id(), true);
        }
        for i in 0..4 {
            self.free_pool[(i + SAVED_REGS) as usize] = (format!("t{}", i + 3).into_id(), true);
        }
        for i in 0..(TEMP_REGS - 4) {
            self.free_pool[(i + SAVED_REGS + 4) as usize] = (format!("a{}", i).into_id(), true);
        }
    }

    fn contains_call(&self, r: Range, calls: &[u32]) -> bool {
        for call in calls {
            if (r.begin..=r.end).contains(call) {
                return true;
            }
        }

        false
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

    fn alloca_register(&mut self, f: Function, r: Range, val: Value, allow_temp: bool) {
        let reg = if allow_temp && self.free_temp_regs > 0 {
            self.alloc_temp_reg()
        } else {
            self.alloc_saved_reg()
        };
        self.places.entry(f).and_modify(|v| {
            v.insert(val, Place::Reg(reg));
        });
        self.active.push((r, val, reg));
        self.sort_active();
    }

    fn sort_active(&mut self) {
        self.active.sort_by(|a, b| a.0.end.cmp(&b.0.end));
    }

    fn alloc_saved_reg(&mut self) -> RegID {
        self.free_saved_regs -= 1;
        for (reg, free) in &mut self.free_pool[0..SAVED_REGS as usize] {
            if *free {
                *free = false;
                return *reg;
            }
        }

        unreachable!()
    }

    fn alloc_temp_reg(&mut self) -> RegID {
        self.free_temp_regs -= 1;
        for (reg, free) in
            &mut self.free_pool[SAVED_REGS as usize..(SAVED_REGS + TEMP_REGS) as usize]
        {
            if *free {
                *free = false;
                return *reg;
            }
        }

        unreachable!()
    }

    fn free_reg(&mut self, reg: RegID) {
        if reg.is_saved_reg() {
            self.free_saved_regs += 1;
        } else {
            self.free_temp_regs += 1;
        }
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
            free_saved_regs: 0,
            free_temp_regs: 0,
            offset: 0,
        }
    }
}
