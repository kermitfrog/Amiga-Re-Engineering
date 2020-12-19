mod cpustep;
mod dump;

extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_big_array;

use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use serde::{Serialize, Deserialize};
//, SizeLimit};
// use bincode::*;
use crate::cpustep::CpuStep;

extern crate rustc_serialize;

fn main() -> std::io::Result<()> {
    /*
        Logic: mode dir val [dir val]{*}
        if !opcode.bin -> transform opcode.log to opcode.bin
     */
    let args: Vec<String> = env::args().collect();



    let file = File::open("/home/harddisk/arek/amiga/ambm/uae-dumps/valdyn_6_dmg/opcode.log")?;
    let mut buf_reader = BufReader::new(file);

    let mut out = File::create("/tmp/dump").unwrap();

    for i in 0..10 {
        let step = CpuStep::from_dump(&mut buf_reader);
        // bincode::encode_into(&step.unwrap(), &mut out, bincode::SizeLimit::Infinite).unwrap();
        // bincode::serialize_into(&mut out, &step.unwrap()).unwrap();
        print!("{}", step.unwrap().to_string());
    }

    Ok(())
}
