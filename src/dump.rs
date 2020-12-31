use crate::cpustep::CpuStep;
use std::collections::{HashMap, BTreeMap, BTreeSet, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use serde::{Serialize, Deserialize};
use std;
use crate::memdump::MemDump;
use crate::utils::FormatHelper;
use std::cmp::min;

#[derive(Serialize, Deserialize)]
pub struct Dump {
    // name: str,
    singles: HashMap<u32, usize>,
    // PC -> Offset
    steps: Vec<CpuStep>,
}

impl Dump {
    pub fn from_dir(path: String) -> std::io::Result<Dump> {
        // let name =
        let file_res = File::open(path.to_owned() + "/opcode.bin");
        match file_res {
            Ok(file) => {
                let buf_reader = BufReader::new(file);
                let dump: Dump = bincode::deserialize_from(buf_reader).expect("Reading failed");
                println!("finished reading");
                Ok(dump)
            }
            Err(_) => {
                let file = File::open(path.to_owned() + "/opcode.log")?;
                let mut buf_reader = BufReader::new(file);

                let mut pcs: HashMap<u32, (u32, usize)> = HashMap::new(); // pc, (count, index)
                let mut singles_all: BTreeMap<u32, usize> = BTreeMap::new();
                let mut singles: HashMap<u32, usize> = HashMap::new();
                let mut steps: Vec<CpuStep> = Vec::new();
                let mut i = 0;
                loop {
                    let step_res = CpuStep::from_dump(&mut buf_reader);
                    match step_res {
                        Ok(step) => {
                            let pc = step.pc; // get Program counter
                            // get entry for pc (or a new one with count = 0)
                            /* this should work - why does is not?
                            let mut e = *pcs.entry(pc).or_insert((0, i));
                            e.0 += 1;
                            so instead, the longer version:
                            */
                            if pcs.contains_key(&pc) {
                                let e = pcs[&pc];
                                pcs.insert(pc, (e.0 + 1, e.1));
                            } else {
                                pcs.insert(pc, (1, i));
                            }

                            steps.push(step);
                            i += 1;
                        }
                        Err(_) => break
                    }
                }
                // get only pcs with count of 1 - in a BTreeMap because we need them sorted
                for (pc, (c, idx)) in pcs.drain() {
                    if c == 1 {
                        singles_all.insert(pc, idx);
                    }
                }

                let mut iter = singles_all.iter();
                let first = iter.next().unwrap();
                let mut pc_last: u32 = *first.0;
                let mut pc_new: u32;
                singles.insert(pc_last, *first.1);
                for (pc, idx) in iter {
                    pc_new = *pc;
                    // if pc_new > pc_last + 10 {
                    if pc_new == steps.get(singles_all[&pc_last]).unwrap().pc_next {
                        singles.insert(*pc, *idx);
                        // println!("({:x}, {})", pc, idx);
                    }
                    pc_last = pc_new;
                }

                let dump = Dump { singles, steps };
                let out = File::create(path.to_owned() + "/opcode.bin")?;
                let mut out_buf = BufWriter::new(out);
                bincode::serialize_into(&mut out_buf, &dump).unwrap();
                Ok(dump)
            }
        }
    }

    pub fn search_for_register_change(&self, val: u32, size: u8, _in_pcs: Option<HashSet<u32>>) -> HashSet<u32> {
        let mask: u32 = match size {
            1 => 0x000000FF,
            2 => 0x0000FFFF,
            _ => 0xFFFFFFFF
        };

        let mut found: HashSet<u32> = HashSet::new();
        for cs in self.singles.values() {
            found.extend(self.search_for_register_change_from(*cs, val, mask));
        }
        found
    }
    pub fn search_for_register_change_from(&self, start: usize, val: u32, mask: u32) -> HashSet<u32> {
        let mut to_go = 1000;
        let mut index = start;
        let mut depth: i16 = 0;
        let mut last: &CpuStep = self.steps.get(index).unwrap();
        let mut found: HashSet<u32> = HashSet::new();
        loop {
            index += 1;
            match self.steps.get(index + 1) {
                Some(current) => {
                    // println!("{}", current.to_string());
                    let mut res = current.register_changed_to(last, val, mask);
                    if res != 0 {
                        found.insert(current.pc);
                    }
                    let mut idx = 0;
                    while res != 0 {
                        if res & 1 == 1 {
                            println!("{:08X}, D{}: {:x} -> {:x} ", current.pc, idx,
                                     last.data[idx], current.data[idx])
                        }
                        res /= 2;
                        idx += 1;
                        // println!("{}", current.to_string());
                    }
                    last = current;
                    to_go -= 1;
                    depth = depth + current.depth_mod();
                    if depth < 0 || to_go <= 0 {
                        break;
                    }
                }
                None => break
            }
        }
        found
    }

    pub fn first_index_of_pc(&self, pc: u32) -> Result<usize, &str> {
        for idx in 0..self.steps.len() {
            if self.steps[idx].pc == pc {
                return Ok(idx);
            }
        }
        Err("PC not found")
    }

    pub fn dump_memlist_cmds(&self, pc: u32, num_before: usize) -> Result<(), &str> {
        let inclusive: u32 = 128;
        let mut addresses: BTreeSet<u32> = BTreeSet::new();
        let end = self.first_index_of_pc(pc)?;
        let start = if end > num_before { end - num_before } else { 0 };
        for idx in start..=end {
            let address = self.steps.get(idx).unwrap().address;
            for i in 0..8 {
                addresses.insert(address[i]);
            }
        }

        let mut iter = addresses.iter();
        let mut first = *iter.next().unwrap();
        let mut last = first;
        let mut new: u32;
        for a in iter {
            new = *a;
            if new > last + inclusive {
                Dump::print_m_range(first, last);
                first = new;
            }
            last = new;
        }
        Ok(())
    }

    fn print_m_range(from: u32, to: u32) {
        let start = from & 0xffffff80;
        let end = (to + 0x100) & 0xffffff80;
        let lines = (end - start) / 16;
        println!("m {:08x} {}", start, lines);
    }

    pub fn inspect(&self, mem: MemDump, pc: u32, num_before: usize, fmt: FormatHelper) -> Result<(), &str> {
        // general preparation
        let end = self.first_index_of_pc(pc)?;
        let start = if end > num_before { end - num_before } else { 0 } + 1;
        let mut current = self.steps.get(start).expect("cpu step not found");
        // get base depth
        let mut depth: i16 = 0;
        let mut min_depth: i16 = 0;
        for i in start..=end {
            depth += self.steps.get(i).expect("cpu step not found").depth_mod();
            min_depth = min(min_depth, depth);
        }
        depth = 0 - min_depth;
        for i in start..=end {
            let last = current;
            current = self.steps.get(i).expect("cpu step not found");
            print!("{}", current.pretty_diff(&last, &mem, &fmt, end - i, &mut depth));
        }
        Ok(())
    }
    pub fn ghidra_search(&self, pc: u32, num_after: usize) -> Result<(), &str> {
        let start = self.first_index_of_pc(pc)?;
        let end = start + num_after;
        let mut current = self.steps.get(start).expect("cpu step not found");

        for i in start..=end {
            let last = current;
            current = self.steps.get(i).expect("cpu step not found");
            match current.print_for_search(&last) {
                Ok(s) => println!("{}", s),
                Err(e) => {
                    println!("{}", e);
                    return Ok(())
                }
            }
        }
        Ok(())
    }

    pub fn stack(&self, pc: u32, fmt: FormatHelper) -> Result<(), &str>{
        let mut idx = self.first_index_of_pc(pc)?;
        let mut depth: i16 = 0;
        let mut min_depth: i16 = 0;
        let mut current = self.steps.get(idx).expect("cpu step not found");
        let mut lines : Vec<String> = Vec::new();
        lines.push(format!("{:08X}  {}", current.pc - fmt.offset_mod,
            std::str::from_utf8(&current.note).unwrap_or_default()));

        loop {
            let last = current;
            current = self.steps.get(idx).expect("cpu step not found");
            depth -= current.depth_mod();
            if depth < min_depth {
                lines.push(format!("{:08X}  {}", last.pc - fmt.offset_mod,
                                   std::str::from_utf8(&last.note).unwrap_or_default()));
                min_depth = depth;
            }
            if idx == 0 {
                break;
            }
            idx -= 1;
        }

        for line in lines.iter().rev() {
            println!("{}", line);
        }
        Ok(())
    }
}