mod cpustep;
mod dump;
mod memdump;

extern crate serde;
extern crate serde_derive;
#[macro_use]
extern crate serde_big_array;

use std::env;
// use std::fs::File;
// use std::io::BufReader;
use crate::dump::Dump;
use crate::memdump::MemDump;

extern crate rustc_serialize;

fn main() -> std::io::Result<()> {
    /*
        Logic: mode dir val [dir val]{*}
        if !opcode.bin -> transform opcode.log to opcode.bin
     */
    let args: Vec<String> = env::args().collect();
    let mut dumps: Vec<Dump> = Vec::new();
    let mut values: Vec<u32> = Vec::new();
    let mode = &args[1];
    match mode.as_str() {
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
            for dump in dumps {
                dump.search_for_register_change(*values.get(i).unwrap(), 2, None);
            }
        }
        "m" => { // get mem info commands :: dump pc num_before
            let path = &args[2];
            let pc = u32::from_str_radix(args[3].as_str(), 16).unwrap();
            let num_before = usize::from_str_radix(args[4].as_str(), 10).unwrap();
            let dump = Dump::from_dir(path.to_string())?;
            dump.dump_memlist_cmds(pc, num_before);
        }
        "md" => {
            let path = &args[2];
            let md = MemDump::from_dir(path.to_string());
        }
        _ => {}
    }

    // let file = File::open("/home/harddisk/arek/amiga/ambm/uae-dumps/valdyn_6_dmg/opcode.log")?;
    // let mut buf_reader = BufReader::new(file);

    // let mut out = File::create("/tmp/dump").unwrap();

    Ok(())
}

