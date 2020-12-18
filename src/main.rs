mod preprocessor;

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use crate::preprocessor::CpuStep;

fn main() -> std::io::Result<()>{
    let file = File::open("/home/harddisk/arek/amiga/ambm/uae-dumps/valdyn_6_dmg/opcode.log")?;
    let mut buf_reader = BufReader::new(file);
    for i in 0..10 {
        let step = CpuStep::from_dump(&mut buf_reader);
        print!("{}", step.unwrap().to_string());
    }
    Ok(())
}
