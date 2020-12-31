mod cpustep;
mod dump;
mod memdump;
mod utils;

extern crate serde;
extern crate serde_derive;
#[macro_use]
extern crate serde_big_array;

use std::env;
// use std::fs::File;
// use std::io::BufReader;
use crate::dump::Dump;
use crate::memdump::MemDump;
use crate::utils::FormatHelper;
use std::fs::File;
use std::io::{BufReader, BufRead};

extern crate rustc_serialize;

fn main() -> std::io::Result<()> {
    /*
        Logic: mode dir val [dir val]{*}
        if !opcode.bin -> transform opcode.log to opcode.bin
     */
    let args: Vec<String> = env::args().collect();
    let mut dumps: Vec<Dump> = Vec::new();
    let mut values: Vec<u32> = Vec::new();
    let mode = if args.len() > 1 { &args[1] } else { "h" };
    match mode {
        "d" => { // search for value in dump :: dir val [dir val] ..
            let mut i = 2;
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
            let i = 0;
            // let mut pcs;
            for dump in dumps {
                dump.search_for_register_change(*values.get(i).unwrap(), 2, None);
            }
        }
        "m" => { // get mem info commands :: dump pc num_before
            let path = &args[2];
            let pc = u32::from_str_radix(args[3].as_str(), 16).unwrap();
            let num_before = usize::from_str_radix(args[4].as_str(), 10).unwrap();
            let dump = Dump::from_dir(path.to_string())?;
            dump.dump_memlist_cmds(pc, num_before).expect("meh!");
        }
        "md" => { //
            let path = &args[2];
            let _md = MemDump::from_dir(path.to_string());
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
        _ => {
            println!("\
           {} [d|m|i|s|g|I|S] parameters\n\
           ... dir   is directory containing dump, named opcode.log\n\
           ... pc    is the programm counter (value displayed above \"Next PC:\" in dump\n\
           ... count is number of instructions before pc\n\n\
           d => search for value (dec) in dump\n\
           $ d dir val [dir val] .. \n\n\
           m => print commands to get memdump for related addresses from fs-uae debugger\n\
                $ m dir pc count\n\n\
           i => print summary of instructions leading to pc (uses linux terminal colors)\n\
                val is value to highlight (format as displayed, pairs of two [0-9,A-Z])\n\
                $ m dir pc count [val]*  | less -R \n\n\
           s => compact version of the above\n\n\
           g => generate ghidra insruction pattern search text for code at pc\n\n\
                $ g dir pc count\n\n\n\
           I|S => like i|s, but subtract value in dir/offset from pc
           The program preprocesses opcode.log to opcode.bin for faster loading.\n\
           If .log or program version has changed, you should delete .bin"
                     , args[0]);
        }
    }

    // let file = File::open("/home/harddisk/arek/amiga/ambm/uae-dumps/valdyn_6_dmg/opcode.log")?;
    // let mut buf_reader = BufReader::new(file);

    // let mut out = File::create("/tmp/dump").unwrap();

    Ok(())
}

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

fn stack(args: &Vec<String>, use_offset: bool) {
    let path = &args[2];
    let pc = u32::from_str_radix(args[3].as_str(), 16).unwrap();
    let offset = if use_offset { get_offset(path) } else { 0 };
    let fmt = FormatHelper::simple(true, 2, offset);

    Dump::from_dir(path.to_string()).expect("could not load dump").stack(pc, fmt);
}

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