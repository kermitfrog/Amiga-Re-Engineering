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
mod cpustep;
mod dump;
mod memdump;
mod utils;

extern crate serde;
extern crate serde_derive;
#[macro_use]
extern crate serde_big_array;

use std::{env, fs};
use crate::dump::Dump;
use crate::memdump::MemDump;
use crate::utils::FormatHelper;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::collections::{BTreeMap, HashMap, BTreeSet};
use core::cmp;

extern crate rustc_serialize;

fn main() -> std::io::Result<()> {
    /*
        Logic: mode dir val [dir val]{*}
        if !opcode.bin -> transform opcode.log to opcode.bin
     */
    let args: Vec<String> = env::args().collect();
    let mode = if args.len() > 1 { &args[1] } else { "h" };
    match mode {
        "d" => { // search for value in dump :: dir val [dir val] ..
            search_value(&args, false);
        }
        "D" => { // search for value in dump :: dir val [dir val] ..
            search_value(&args, true);
        }
        "m" => { // get mem info commands :: dump pc num_before
            let path = &args[2];
            let pc = u32::from_str_radix(args[3].as_str(), 16).unwrap();
            let num_before = usize::from_str_radix(args[4].as_str(), 10).unwrap();
            let dump = Dump::from_dir(path.to_string())?;
            dump.dump_memlist_cmds(pc, num_before).expect("meh!");
        }
        "md" => { // parse memdump -- this is just for testing
            let path = &args[2];
            let md = MemDump::from_dir(path.to_string());
            println!("{}", md.unwrap().to_string());
        }
        "M" => {
            map_data_to_mem(&args)
        }
        "i" => { // inspect.. dir pc pre [highlight str]*
            summary(&args, false, false)
        }
        "s" => { // summary dir pc pre [highlight str]*
            summary(&args, true, false);
        }
        "I" => { // inspect.. dir pc pre [highlight str]*
            summary(&args, false, true)
        }
        "S" => { // summary dir pc pre [highlight str]*
            summary(&args, true, true);
        }
        "g" => {
            let path = &args[2];
            let pc = u32::from_str_radix(args[3].as_str(), 16).unwrap();
            let num_after = usize::from_str_radix(args[4].as_str(), 10).unwrap();
            Dump::from_dir(path.to_string()).expect("could not load dump")
                .ghidra_search(pc, num_after).expect("generating search pattern failed");
        }
        "t" => {
            stack(&args, false);
        }
        "T" => {
            stack(&args, true);
        }
        "c" => {
            show_calls(&args, false);
        }
        "C" => {
            show_calls(&args, true);
        }
        "p" => {
            let path = &args[2];
            Dump::from_dir(path.to_string()).expect("could not load dump")
                .starting_pcs(0);
        }
        "P" => {
            let path = &args[2];
            Dump::from_dir(path.to_string()).expect("could not load dump")
                .starting_pcs(get_offset(path));
        }
        "io" => {
            in_out_state(&args, false);
        }
        "IO" => {
            in_out_state(&args, true);
        }
        "sd" => {
            mem_set_diff(&args);
        }
        _ => {
            println!("\
           {} [d|m|i|s|g|p|D|I|S|M|P|t|T|io|IO] parameters\n\
           ... dir   is directory containing dump, named opcode.log\n\
           ... pc    is the programm counter (value displayed above \"Next PC:\") in dump\n\
           ... count is number of instructions before pc\n\n\
           d => search for value (dec) in dump\n\
           $ d dir val [dir val] .. \n\n\
           m => print commands to get memdump for related addresses from fs-uae debugger\n\
                $ m dir pc count\n\n\
           i => print summary of instructions leading to pc (uses linux terminal colors)\n\
                val is value to highlight (format as displayed, pairs of two [0-9,A-Z])\n\
           $ i dir pc count [val]*  | less -R \n\n\
           s => compact version of the above\n\n\
           g => generate ghidra insruction pattern search text for code at pc\n\
                $ g dir pc count_after\n\n\
           p => print starting pcs\n\
                $ p dir\n\n\
           M => map data to memory dump - finds locations of files in data_dir in memory dump, ignoring data with < 8 non-zero bytes\n\
                $ M dir data_dir > dataMap.csv\n\n\
           t => print call hierarchy leading to pc\n\
                $ t dir pc\n\n\
           io => print register states at specific pcs\n\
                $ io dir pc_start pc_end\n\n\
           sd => print differences between sets of memory dumps. dir contains directories named set_id\n\
                $ sd dir\n\n\
           D|I|S|P|T => like d|i|s|p|t, but subtract value in dir/offset (one line, hex, no 0x) from pc\n\
           IO => like io, but add offset value to parameters\n\
           Do NOT rely on printed memory content! The values are at the time, the memory dump was made\n\
           and might have changed since then!\n\
           The program preprocesses opcode.log to opcode.bin for faster loading.\n\
           If .log or program version has changed, you might want to delete .bin"
                     , args[0]);
        }
    }

    // let file = File::open("/home/harddisk/arek/amiga/ambm/uae-dumps/valdyn_6_dmg/opcode.log")?;
    // let mut buf_reader = BufReader::new(file);

    // let mut out = File::create("/tmp/dump").unwrap();

    Ok(())
}

/// print short version of steps leading to pc
///
/// short: if true, use one line version without highlighting
fn summary(args: &Vec<String>, short: bool, use_offset: bool) {
    let path = &args[2];
    let pc = u32::from_str_radix(args[3].as_str(), 16).unwrap();
    let num_before = usize::from_str_radix(args[4].as_str(), 10).unwrap();
    let mut highlight: Vec<String> = Vec::new();
    for i in 5..args.len() {
        highlight.push(args[i].to_owned());
    }
    let offset = if use_offset { get_offset(path) } else { 0 };
    println!("{}", offset);

    let fmt = if short {
        FormatHelper::simple(true, 2, offset)
    } else {
        FormatHelper::for_values(&highlight, false, 2, offset)
    };
    let mem: MemDump = match MemDump::from_dir(path.to_string()) {
        Ok(m) => m,
        Err(_) => MemDump::new()
    };
    Dump::from_dir(path.to_string()).expect("could not load dump")
        .inspect(mem, pc, num_before, fmt).expect("summary failed");
}

/// print call hierarchy leading to pc
fn stack(args: &Vec<String>, use_offset: bool) {
    let path = &args[2];
    let pc = u32::from_str_radix(args[3].as_str(), 16).unwrap();
    let offset = if use_offset { get_offset(path) } else { 0 };
    let fmt = FormatHelper::simple(true, 2, offset);

    Dump::from_dir(path.to_string()).expect("could not load dump").stack(pc, fmt)
        .expect("failed reading dump ");
}

/// print complete call hierarchy
fn show_calls(args: &Vec<String>, use_offset: bool) {
    let path = &args[2];
    let offset = if use_offset { get_offset(path) } else { 0 };
    let fmt = FormatHelper::simple(true, 2, offset);

    Dump::from_dir(path.to_string()).expect("could not load dump").calls(fmt)
        .expect("failed reading dump ");
}

/// load offset from path/offset or 0 if file is missing
fn get_offset(path: &String) -> u32 {
    let file_offset = File::open(path.to_owned() + "/offset");
    match file_offset {
        Ok(file) => {
            let mut buf_reader = BufReader::new(file);
            let mut s = String::new();
            let _ = buf_reader.read_line(&mut s);
            u32::from_str_radix(&s.trim_end(), 16).unwrap_or_default()
        }
        Err(_) => 0u32
    }
}

/// search Dumps for a register change to a specific value
/// multiple dumps (with one value each) can be specified, in which case only results that make the
/// change at the same program counter in each dump are printed
fn search_value(args: &Vec<String>, use_offset: bool) {
    let mut dumps: Vec<Dump> = Vec::new();
    let mut values: Vec<u32> = Vec::new();
    let mut i = 2;
    let offset = if use_offset { get_offset(&args[2]) } else { 0 };
    loop {
        let path = &args[i];
        let dump_r = Dump::from_dir(path.to_string());
        match dump_r {
            Ok(dump) => { dumps.push(dump); }
            Err(_) => { println!("ERROR"); }
        }
        // let dump = Dump::from_dir(path.to_string())?;
        // dumps.push(dump);
        values.push(u32::from_str_radix(args[i + 1].as_str(), 10).unwrap_or_default());
        i += 2;
        if i >= args.len() - 1 {
            break;
        }
    }
    let mut i = 0;
    let mut results: Option<BTreeMap<u32, String>> = None;
    let mut size: u8 = 1;
    for val in &values {
        size = cmp::max(size, match val {
            0..=0xFF => 1,
            0x100..=0xFF00 => 2,
            _ => 4
        });
    }

    for dump in dumps {
        results = Some(dump.search_for_register_change(*values.get(i).unwrap(), size, results));
        i += 1;
    }
    for (k, v) in results.unwrap_or_default() {
        println!("{:08X}{}", k - offset, v);
    }
}

fn map_data_to_mem(args: &Vec<String>) {
    if args.len() < 4 {
        return;
    }
    let dump_dir = &args[2];
    let data_dir = &args[3];

    let offset = get_offset(&dump_dir.to_string());
    let md = MemDump::from_dir(dump_dir.to_string()).expect("could not load memory");
    md.map_data(data_dir.to_string(), offset).unwrap();
}

fn in_out_state(args: &Vec<String>, use_offset: bool) {
    let path = &args[2];
    let dump = Dump::from_dir(path.to_string()).expect("could not load dump");
    let offset = if use_offset { get_offset(&args[2]) } else { 0 };
    let start = u32::from_str_radix(&args[3], 16).expect("could not parse start") + offset;
    let end = u32::from_str_radix(&args[4], 16).expect("could not parse end") + offset;
    dump.in_out_state(start, end);
}

/// check sets of memory dumps for bytes that differ between sets, but not inside them
fn mem_set_diff(args: &Vec<String>) { // TODO improve error messages
    let entries = fs::read_dir(&args[2]).expect("could not open dir");
    let mut memdump_map: HashMap<String, Vec<MemDump>> = HashMap::new();
    let offset = get_offset(&args[2]);

    // load memdumps and group them by the part of filename before '_'
    for entry in entries {
        let entry = entry.expect("something wrong with entry");
        let path = entry.path();
        let file_name = entry.file_name();
        let name_parts : Vec<&str> = file_name.to_str().unwrap_or_default().split('_').collect();

        if name_parts.len() == 2 {
            let key = name_parts[0];
            let mem = MemDump::from_dir(path.to_str().expect("something wrong with path").to_string()).expect("could not load mem dump");
            let val = memdump_map.get_mut(key);
            match val {
                Some(e) => {
                    e.push(mem);
                }
                None => {
                    let mut vec : Vec<MemDump> = Vec::new();
                    vec.push(mem);
                    memdump_map.insert(key.to_owned(), vec);
                }
            }
        }
    }

    // no need to have a map anymore - transfer to a Vec
    let mut memdump_vec : Vec<Vec<MemDump>> = Vec::new();
    for (_, val) in memdump_map.drain() {
        memdump_vec.push(val);
    }

    // look up offsets that are different between each set
    // only the first memdump per set is checked, as the interesting parts are identical in each
    // memdump of a set and the false offsets will be filtered out later
    let mut results : Option<BTreeSet<u32>> = None; // we use a sorted set, to get a sorted output
    for i in 0..memdump_vec.len() -1 {
        // the first diff will check all memory, while subsequent diffs only need to check the
        // offsets in results
        for j in i+1..memdump_vec.len() {
            results = Some(memdump_vec[i][0].diff_only(&memdump_vec[j][0], results, false));
        }
    }

    // filter out bytes that change inside a set
    for vec in memdump_vec {
        for i in 0..vec.len() - 1 {
            for j in i+1..vec.len() {
                results = Some(vec[i].diff_only(&vec[j], results, true));
            }
        }
    }

    // output
    if offset == 0 {
        for r in results.expect("No Result") {
            println!("{:08X}", r);
        }
    } else {
        for r in results.expect("No Result") {
            println!("{:08X}  {:08X}", r.wrapping_sub(offset), r);
        }
    }

}