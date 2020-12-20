mod cpustep;
mod dump;

extern crate serde;
extern crate serde_derive;
#[macro_use]
extern crate serde_big_array;

use std::env;
// use std::fs::File;
// use std::io::BufReader;
use crate::dump::Dump;

extern crate rustc_serialize;

fn main() -> std::io::Result<()> {
    /*
        Logic: mode dir val [dir val]{*}
        if !opcode.bin -> transform opcode.log to opcode.bin
     */
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        return Ok(());
    }
    let mut dumps: Vec<Dump> = Vec::new();
    let mut values: Vec<u32> = Vec::new();
    let mode = &args[1];
    if mode == "d" {
        let mut i = 2;
        loop {
            let path = &args[i];
            let dump_r = Dump::from_dir(path.to_string());
            match dump_r {
                Ok(dump) => { dumps.push(dump); }
                Err(_) => { println!("ERROR");}
            }
            // let dump = Dump::from_dir(path.to_string())?;
            // dumps.push(dump);
            values.push(u32::from_str_radix(args[i+1].as_str(), 10).unwrap_or_default());
            i += 2;
            if i >= args.len() - 1 {
                break;
            }
        }

    }

    // let file = File::open("/home/harddisk/arek/amiga/ambm/uae-dumps/valdyn_6_dmg/opcode.log")?;
    // let mut buf_reader = BufReader::new(file);

    // let mut out = File::create("/tmp/dump").unwrap();

    Ok(())
}
