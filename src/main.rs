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

use std::{fs};
use crate::dump::Dump;
use crate::memdump::MemDump;
use crate::utils::{FormatHelper};
use std::collections::{BTreeMap, HashMap, BTreeSet};
use core::cmp;
use clap::{Arg, App, ArgMatches, ValueHint, AppSettings};

extern crate rustc_serialize;

fn main() -> std::io::Result<()> {
    /*
        Logic: mode dir val [dir val]{*}
        if !opcode.bin -> transform opcode.log to opcode.bin
     */
    let matches = App::new("Dump-Analyzer")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .author("Arkadiusz Guzinski <kermit@ag.de1.cc>")
        .about("Helps you make sense of hacked FS-UAE instruction dump\nFor most commands\n\
           ... dir   is a directory containing the associated data, see help-fs for filename details\n\
           ... pc    is the program counter (value displayed above \"Next PC:\") in hex\n\
           ... count is number of instructions before|after pc (depending on subcommand)
        ")
        .arg(Arg::new("compact").short('s').global(true)
            .about("use compact mode for summary")
        )
        .arg(Arg::new("colors").short('c').global(true)
            .about("use console colors")
        )
        .arg(Arg::new("no-colors").short('b').global(true)
            .about("disable console colors").conflicts_with("colors")
        )
        .arg(Arg::new("indent").short('i').global(true)
            .about("intent by [num] spaces")
            .takes_value(true)
        )
        .arg(Arg::new("offset-mode").short('o').global(true)
            .about("print program counters as..")
            .possible_values(&["dump", "translated", "both"])
        )
        .arg(Arg::new("function-names").short('n').global(true)
            .about("print function names if possible")
            .possible_values(&["never", "entry", "always"])
        )
        .arg(Arg::new("traps").short('t').global(true)
            .about("show interrupts (traps)")
        )

        .subcommand(App::new("search-value").visible_aliases(&["d", "D"])
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("searches for a value (dec) in one or more dumps")
            .arg(Arg::new("dir val").multiple(true).min_values(2).required(true)
                .about("pairs of directory with dump and a search value in decimal")
            )
        )

        .subcommand(App::new("print-mem-commands").visible_alias("m")
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("print commands to get memdump only for related addresses from fs-uae debugger")
            .long_about("print commands to get memdump only for related addresses from fs-uae debugger\n\
                This may be useful if you want to look at the memory yourself.\n\
                For getting all memory it's probably better to simply use the 'S' debugger command")
            .arg(Arg::new("dir").required(true).index(1)
                .value_hint(ValueHint::DirPath)
                .about("directory containing memory dump")
            )
            .arg(Arg::new("pc").required(true).index(2)
                .value_hint(ValueHint::Other)
                .about("program counter (hex)")
            )
            .arg(Arg::new("count").required(true).index(3)
                .value_hint(ValueHint::Other)
                .about("lines of memory around the actual data")
            )
        )

        .subcommand(App::new("summary-long").visible_aliases(&["i", "I"])
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("print summary of instructions leading to pc (uses linux terminal colors)")
            .arg(Arg::new("dir").required(true).index(1)
                .value_hint(ValueHint::DirPath)
                .about("directory containing the dump")
            )
            .arg(Arg::new("pc").required(true).index(2)
                .about("program counter (hex)")
                .value_hint(ValueHint::Other)
            )
            .arg(Arg::new("count").required(true).index(3)
                .about("number of instructions to print before pc")
                .value_hint(ValueHint::Other)
            )
            .arg(Arg::new("val").multiple(true).index(4)
                .about("values to highlight; format is as printed (hex)")
                .value_hint(ValueHint::Other)
            )
        )

        .subcommand(App::new("summary").visible_aliases(&["s", "S"])
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("print compact summary of instructions leading to pc")
            .arg(Arg::new("dir").required(true).index(1)
                .value_hint(ValueHint::DirPath)
                .about("directory containing the dump")
            )
            .arg(Arg::new("pc").required(true).index(2)
                .about("program counter (hex)")
                .value_hint(ValueHint::Other)
            )
            .arg(Arg::new("count").required(true).index(3)
                .about("number of instructions to print before pc")
                .value_hint(ValueHint::Other)
            )
            .arg(Arg::new("val").multiple(true).index(4)
                .about("values to highlight; format is as printed (hex)")
                .value_hint(ValueHint::Other)
            )
        )

        .subcommand(App::new("print-ghidra-search-pattern").visible_alias("g")
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("generate ghidra instruction pattern search text for code at pc")
            .arg(Arg::new("dir").required(true).index(1)
                .about("directory containing the dump")
                .value_hint(ValueHint::DirPath)
            )
            .arg(Arg::new("pc").required(true).index(2)
                .about("program counter (hex)")
                .value_hint(ValueHint::Other)
            )
            .arg(Arg::new("count").index(3)
                .about("number of instructions to print after pc (default: 30)")
                .value_hint(ValueHint::Other)
            )
        )

        .subcommand(App::new("starting-pcs").visible_aliases(&["p", "P"])
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("print starting pcs for functions that are called just once in dump")
            .arg(Arg::new("dir").required(true).index(1)
                .about("directory containing the dump")
                .value_hint(ValueHint::DirPath)
            )
        )

        .subcommand(App::new("map-data").visible_alias("M")
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("map data to memory dump")
            .arg(Arg::new("dir").required(true).index(1)
                .about("directory containing the memory dump")
                .value_hint(ValueHint::DirPath)
            )
            .arg(Arg::new("data-dir").required(true).index(2)
                .value_hint(ValueHint::DirPath)
            )
        )

        .subcommand(App::new("stack").visible_aliases(&["t", "T"])
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("print call hierarchy leading to pc")
            .arg(Arg::new("dir").required(true).index(1)
                .about("directory containing the dump")
                .value_hint(ValueHint::DirPath)
            )
            .arg(Arg::new("pc").required(true).index(2)
                .about("program counter (hex)")
                .value_hint(ValueHint::Other)
            )
        )

        .subcommand(App::new("registers").visible_aliases(&["io", "IO"])
            .setting(AppSettings::ArgRequiredElseHelp)
            .about(" print register states at specific pcs")
            .arg(Arg::new("dir").required(true).index(1)
                .about("directory containing the dump")
                .value_hint(ValueHint::DirPath)
            )
            .arg(Arg::new("pc_start").required(true).index(2)
                .about("program counter at start of a function (hex)")
                .value_hint(ValueHint::Other)
            )
            .arg(Arg::new("pc_end").required(true).index(3)
                .about("program counter at end of a function (hex)")
                .value_hint(ValueHint::Other)
            )
        )

        .subcommand(App::new("memset-diff").visible_alias("sd")
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("print differences between sets of memory dumps. dir contains directories named set_id")
            .arg(Arg::new("set_dir").required(true).index(1)
                .about("dir containing directories named set_id (see help-fs)")
                .value_hint(ValueHint::DirPath)
            )
        )

        .subcommand(App::new("calls").visible_aliases(&["c", "C"])
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("print call hierarchy of dump")
            .arg(Arg::new("dir").required(true).index(1)
                .about("directory containing the dump")
                .value_hint(ValueHint::DirPath)
            )
        )

        .subcommand(App::new("help-fs").about("print info about the expected file structure"))

        .get_matches();

    match matches.subcommand() {
        Some(("calls", sub_args)) => show_calls(&sub_args),
        Some(("help-fs", _)) => print_help_fs(),
        Some(("map-data", sub_args)) => map_data_to_mem(&sub_args),
        Some(("memset-diff", sub_args)) => mem_set_diff(&sub_args),
        Some(("print-ghidra-search-pattern", sub_args)) => print_ghidra_search_pattern(&sub_args),
        Some(("print-mem-commands", sub_args)) => print_mem_commands(&sub_args), // get mem info commands :: dump pc num_before
        Some(("registers", sub_args)) => in_out_state(&sub_args),
        Some(("search-value", sub_args)) => search_value(&sub_args), // search for value in dump :: dir val [dir val] ..
        Some(("stack", sub_args)) => stack(&sub_args),
        Some(("starting-pcs", sub_args)) => print_starting_pcs(&sub_args),
        Some(("summary", sub_args)) => summary(&sub_args, true), // inspect dir pc pre [highlight str]*
        Some(("summary-long", sub_args)) => summary(&sub_args, false), // summary dir pc pre [highlight str]*
        _ => println!("Unknown")
    }
    Ok(())
}

fn print_help_fs() {
    println!("{}",
    "This program expects the data to be inside a directory, following a specific naming scheme\n\
     dir contains:\n\
         opcode.log   this is the instruction dump, as generated by the modified FS-UAE.\n\
         opcode.bin   is the above, preprocessed to a binary format for faster loading.
             it may be necessary to delete after an update of the program or opcode.log\n\
         offset       file containing only one hex value that can be subtracted from the program
             counter to print the direct address for Ghidra or similar tools.\n\
         mem          memory dump in text form\n\
         [0-9,A-F]{8} binary memory dump - the preferred way.. the name is a 8-digit hexadecimal
             value, equal to the starting address, e.g. 00000000 or 07000000\n\
         functions.xml  Ghidra xml export, containing function information.\n\
\n\
offset and functions.xml will be used from parent of dir, if not found\n\
\n\
    For the memset-diff command, set_dir expects a directory, containing directories with memory\n\
    dumps, all applying to the same range of memory. These directories are named set_id, where
    set          is the name of the set
    id           can be anything not containing underscores\n\
    Then the command will print the positions of values that differ between all memory dumps of\n\
    different sets, but are equal within a set."
    )
}

/*
           {} [d|m|i|s|g|p|D|I|S|M|P|t|T|io|IO] parameters\n\
           ... dir   is directory containing dump, named opcode.log\n\
           ... pc    is the program counter (value displayed above \"Next PC:\") in dump\n\
           ... count is number of instructions before pc\n\n\
           d => search for value (dec) in dump\n\
           $ d dir val [dir val] .. \n\n\
           m => print commands to get memdump for related addresses from fs-uae debugger\n\
                $ m dir pc count\n\n\
           i => print summary of instructions leading to pc (uses linux terminal colors)\n\
                val is value to highlight (format as displayed, pairs of two [0-9,A-Z])\n\
           $ i dir pc count [val]*  | less -R \n\n\
           s => compact version of the above\n\n\
           g => generate ghidra instruction pattern search text for code at pc\n\
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
*/

/// print short version of steps leading to pc
///
/// short: if true, use one line version without highlighting
fn summary(args: &ArgMatches, short: bool) {
    let path = args.value_of("dir").unwrap_or_default();
    let pc = u32::from_str_radix(args.value_of("pc").unwrap(), 16).unwrap();
    let num_before = usize::from_str_radix(args.value_of("count").unwrap(), 10).unwrap();
    let highlight = args.values_of("val").unwrap_or_default();

    let fmt = if short {
        FormatHelper::simple(true)
    } else {
        FormatHelper::for_values(&highlight, false)
    }.finalize(args);
    let mem: MemDump = match MemDump::from_dir(path.to_string()) {
        Ok(m) => m,
        Err(_) => MemDump::new()
    };
    Dump::from_dir(path.to_string()).expect("could not load dump")
        .inspect(mem, pc, num_before, fmt).expect("summary failed");
}

/// print call hierarchy leading to pc
fn stack(args: &ArgMatches) {
    let path = args.value_of("dir").unwrap();
    let pc = u32::from_str_radix(args.value_of("pc").unwrap(), 16).unwrap();
    let fmt = FormatHelper::simple(true).finalize(args);

    Dump::from_dir(path.to_string()).expect("could not load dump").stack(pc, fmt)
        .expect("failed reading dump ");
}

/// print complete call hierarchy
fn show_calls(args: &ArgMatches) {
    let path = args.value_of("dir").unwrap();
    let fmt = FormatHelper::simple(true).finalize(args);

    Dump::from_dir(path.to_string()).expect("could not load dump").calls(fmt)
        .expect("failed reading dump ");
}

/// search Dumps for a register change to a specific value
/// multiple dumps (with one value each) can be specified, in which case only results that make the
/// change at the same program counter in each dump are printed
fn search_value(args: &ArgMatches) {
//    args: &Vec<String>, use_offset: bool) {
    let mut dumps: Vec<Dump> = Vec::new();
    let mut values: Vec<u32> = Vec::new();
    let offset = FormatHelper::get_offset(&args);
    let mut dir_val = args.values_of("dir_val").unwrap_or_default();
    if dir_val.len() % 2 == 1 {
        println!("value missing for dir");
        return;
    }
    while let Some(path) = dir_val.next() {
        let dump_r = Dump::from_dir(path.to_string());
        match dump_r {
            Ok(dump) => { dumps.push(dump); }
            Err(_) => { println!("ERROR"); }
        }
        // let dump = Dump::from_dir(path.to_string())?;
        // dumps.push(dump);
        values.push(u32::from_str_radix(dir_val.next().unwrap_or_default(), 10).unwrap_or_default());
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
        println!("{:08X}{}", k - offset, v); // TODO use FormatHelper
    }
}

fn map_data_to_mem(args: &ArgMatches) {
    let dump_dir = args.value_of("dir").unwrap();
    let data_dir = args.value_of("data-dir").unwrap();

    let offset = FormatHelper::get_offset(&args);
    let md = MemDump::from_dir(dump_dir.to_string()).expect("could not load memory");
    md.map_data(data_dir.to_string(), offset).unwrap();
}

fn in_out_state(args: &ArgMatches) {
    let path = args.value_of("dir").unwrap();
    let dump = Dump::from_dir(path.to_string()).expect("could not load dump");
    let offset = FormatHelper::get_offset(args);
    // TODO use FormatHelper
    let start = u32::from_str_radix(args.value_of("pc_start").unwrap(), 16)
        .expect("could not parse start") + offset;
    let end = u32::from_str_radix(args.value_of("pc_end").unwrap(), 16)
        .expect("could not parse end") + offset;
    dump.in_out_state(start, end);
}

fn print_mem_commands(args: &ArgMatches) {
    Dump::from_dir(args.value_of("dir").unwrap_or_default().to_string())
        .expect("failed to load dump")
        .dump_memlist_cmds(u32::from_str_radix(args.value_of("pc")
                                                   .unwrap_or_default(), 16).expect("invalid value for pc"),
                           usize::from_str_radix(args.value_of("count").unwrap_or_default(), 10)
                               .expect("invalid value for count"))
        .expect("meh!")
}

/// check sets of memory dumps for bytes that differ between sets, but not inside them
fn mem_set_diff(args: &ArgMatches) { // TODO improve error messages
    let entries = fs::read_dir(args.value_of("set_dir").unwrap())
        .expect("could not open dir");
    let mut memdump_map: HashMap<String, Vec<MemDump>> = HashMap::new();
    let offset = FormatHelper::get_offset(args);

    // load memdumps and group them by the part of filename before '_'
    for entry in entries {
        let entry = entry.expect("something wrong with entry");
        let path = entry.path();
        let file_name = entry.file_name();
        let name_parts: Vec<&str> = file_name.to_str().unwrap_or_default().split('_').collect();

        if name_parts.len() == 2 {
            let key = name_parts[0];
            let mem = MemDump::from_dir(path.to_str().expect("something wrong with path").to_string()).expect("could not load mem dump");
            let val = memdump_map.get_mut(key);
            match val {
                Some(e) => {
                    e.push(mem);
                }
                None => {
                    let mut vec: Vec<MemDump> = Vec::new();
                    vec.push(mem);
                    memdump_map.insert(key.to_owned(), vec);
                }
            }
        }
    }

    // no need to have a map anymore - transfer to a Vec
    let mut memdump_vec: Vec<Vec<MemDump>> = Vec::new();
    for (_, val) in memdump_map.drain() {
        memdump_vec.push(val);
    }

    // look up offsets that are different between each set
    // only the first memdump per set is checked, as the interesting parts are identical in each
    // memdump of a set and the false offsets will be filtered out later
    let mut results: Option<BTreeSet<u32>> = None; // we use a sorted set, to get a sorted output
    for i in 0..memdump_vec.len() - 1 {
        // the first diff will check all memory, while subsequent diffs only need to check the
        // offsets in results
        for j in i + 1..memdump_vec.len() {
            results = Some(memdump_vec[i][0].diff_only(&memdump_vec[j][0], results, false));
        }
    }

    // filter out bytes that change inside a set
    for vec in memdump_vec {
        for i in 0..vec.len() - 1 {
            for j in i + 1..vec.len() {
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

fn print_ghidra_search_pattern(args: &ArgMatches) {
    let path = args.value_of("dir").unwrap();
    let pc = u32::from_str_radix(args.value_of("pc").unwrap(), 16).unwrap();
    let num_after = usize::from_str_radix(
        args.value_of("count").unwrap_or("30"), 10).unwrap();
    Dump::from_dir(path.to_string()).expect("could not load dump")
        .ghidra_search(pc, num_after).expect("generating search pattern failed");
}

fn print_starting_pcs(args: &ArgMatches) {
    let path = args.value_of("dir").unwrap();
    Dump::from_dir(path.to_string()).expect("could not load dump")
        .starting_pcs(0); // TODO use FormatHelper
}