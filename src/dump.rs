/*
    Copyright (C) 2020 Arkadiusz Guzinski <kermit@ag.de1.cc>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use crate::cpustep::CpuStep;
use std::collections::{HashMap, BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use serde::{Serialize, Deserialize};
use std;
use crate::memdump::MemDump;
use crate::utils::FormatHelper;
use std::cmp::min;

#[derive(Serialize, Deserialize)]
/// represents an uae instruction dump
pub struct Dump {
    // name: str,
    /// map of program counters, that are only found once in the dump. These are likely what the
    /// user is searching for. The second value is the number of consecutive pcs, starting with the
    /// first value.
    singles: HashMap<u32, usize>,
    // PC -> Offset
    /// the individual instructions and their register contents
    steps: Vec<CpuStep>,
}

impl Dump {
    /// loads the Dump from the directory, specified by path. Reads a cache file named opcode.bin, or
    /// creates it from opcode.log
    pub fn from_dir(path: String) -> std::io::Result<Dump> {
        // let name =
        let file_res = File::open(path.to_owned() + "/opcode.bin");
        match file_res {
            Ok(file) => {
                let buf_reader = BufReader::new(file);
                let dump: Dump = bincode::deserialize_from(buf_reader).expect("Reading failed");
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
                    if pc_new != steps.get(singles_all[&pc_last]).unwrap().pc_next {
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

    /// Searches individual dumped instruction for a data change to value val.
    ///
    /// If previous is not None, the result will only contain changes that were present at pcs in
    /// previous, as well as those found in this self.
    /// The function does not search the whole dump, but instead searches beginning from each key
    /// of self.singles and stops when reaching a lower call depth then it started with
    ///
    /// val: value to search for
    /// size: expected size of value in bytes (1, 2, anything else will search 4 bytes)
    /// previous: should be result of the last call to this function
    ///
    /// returns: Sorted Map of pc to String describing register changes
    pub fn search_for_register_change(&self, val: u32, size: u8, previous: Option<BTreeMap<u32, String>>)
                                      -> BTreeMap<u32, String> {
        let mask: u32 = match size {
            1 => 0x000000FF,
            2 => 0x0000FFFF,
            _ => 0xFFFFFFFF
        };

        let mut found: BTreeMap<u32, String> = BTreeMap::new();
        for cs in self.singles.values() {
            found.extend(self.search_for_register_change_from(*cs, val, mask));
        }

        match previous {
            None => found,
            Some(found_earlier) => {
                let mut result: BTreeMap<u32, String> = BTreeMap::new();
                for key in found_earlier.keys() {
                    if found.contains_key(key) {
                        let i = *key;
                        result.insert(i, found_earlier[&i].to_string() + found[&i].as_str());
                    }
                }
                result
            }
        }
    }

    /// Finds register changes to value val
    ///
    /// start: start at steps[start]
    /// val: value to look for
    /// mask: bitmask for value (all saved values are 32 bit)
    ///
    /// returns: Map of pc, description of change (e.g. ", D0: 15 -> 14")
    fn search_for_register_change_from(&self, start: usize, val: u32, mask: u32)
                                           -> BTreeMap<u32, String> {
        // maximum of instructions to search
        let mut to_go = 10000;
        let mut index = start;
        // we track depth, so we can return when reaching the function, that called the one at start
        let mut depth: i16 = 0;
        let mut last: &CpuStep = self.steps.get(index).unwrap();
        let mut found: BTreeMap<u32, String> = BTreeMap::new();
        loop {
            index += 1;
            match self.steps.get(index + 1) {
                Some(current) => {
                    // println!("{}", current.to_string());
                    let mut res = current.register_changed_to(last, val, mask);
                    let mut idx = 0;
                    if res != 0 {
                        let mut s = String::new();
                        while res != 0 {
                            if res & 1 == 1 {
                                s += format!(", @{} D{}: {:x} -> {:x} ", index, idx,
                                             last.data[idx], current.data[idx]).as_str();
                            }
                            res /= 2;
                            idx += 1;
                            // println!("{}", current.to_string());
                        }
                        found.insert(current.pc, s);
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

    /// finds first index of pc in self.steps
    fn first_index_of_pc(&self, pc: u32) -> Result<usize, &str> {
        for idx in 0..self.steps.len() {
            if self.steps[idx].pc == pc {
                return Ok(idx);
            }
        }
        Err("PC not found")
    }

    /// tries to create commands, to get memdumps from uae's debug mode, containing all memory at
    /// the addresses that were in address registers at some time (plus some padding for context).
    /// So far, only works for memory directly in address registers, not for directly specified or
    /// accessed with offset
    pub fn dump_memlist_cmds(&self, pc: u32, num_before: usize) -> Result<(), &str> {
        // find addresses
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

        // create ranges, containing those addresses
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

    /// prints command for dump_memlist_cmds
    fn print_m_range(from: u32, to: u32) {
        let start = from & 0xffffff80;
        let end = (to + 0x100) & 0xffffff80;
        let lines = (end - start) / 16;
        println!("m {:08x} {}", start, lines);
    }

    /// prints a summary of instructions and data changes, leading to pc
    ///
    /// mem: MemDump, that can (partially) resolve references in address registers (can be empty)
    /// pc: program counter at which to start (first occurrence in dump will be used)
    /// num_before: print a maximum of num_before instructions prior to pc
    /// fmt: contains formatting options
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

    /// print a string that can be pasted into ghidra's instruction search (hex mode)
    ///
    /// pc: instruction to start at
    /// num_after: include this many following instructions
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
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// print call hierarchy, leading to pc
    ///
    /// pc: program counter at the bottom of the hierarchy (first occurrence in dump will be used)
    /// fmt: contains formatting options
    pub fn stack(&self, pc: u32, fmt: FormatHelper) -> Result<(), &str> {
        let mut idx = self.first_index_of_pc(pc)?;
        let mut depth: i16 = 0;
        let mut min_depth: i16 = 0;
        let mut current = self.steps.get(idx).expect("cpu step not found");
        let mut lines: Vec<String> = Vec::new();
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

    /// print starting points found in dump
    pub fn starting_pcs(&self, offset: u32) {
        for pc in self.singles.keys() {
            println!("{:08X}", *pc - offset);
        }
    }
}