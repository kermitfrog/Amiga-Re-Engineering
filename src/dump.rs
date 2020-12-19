use crate::cpustep::CpuStep;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use bincode::Error;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Dump {
    // name: str,
    singles: HashMap<u32, u64>, // PC -> Offset
    steps: Vec<CpuStep>,
}

impl Dump {
    pub fn from_dir(path: String) -> Result<Dump, Error> {
        // let name =
        let mut file_res = File::open(path.to_owned() + "/opcode.bin");
        match file_res {
            Ok(file) => {
                let mut dump: Dump = bincode::deserialize_from(file)?;
                Ok((dump))
            }
            Err(_) => {
                let file = File::open(path.to_owned() + "/opcode.log")?;
                let mut buf_reader = BufReader::new(file);
                let mut out = File::create(path.to_owned() + "/opcode.bin")?;

                let mut pcs: HashMap<u32, (u32, u64)> = HashMap::new(); // pc, (count, index)
                let mut singles: HashMap<u32, u64> = HashMap::new();
                let mut steps: Vec<CpuStep> = Vec::new();
                let mut i = 0;
                loop {
                    let step_res = CpuStep::from_dump(&mut buf_reader);
                    match step_res {
                        Ok(step) => {
                            let pc = step.pc; // get Program counter
                            // get entry for pc (or a new one with count = 0)
                            let (mut c, idx) = *pcs.entry(pc).or_insert((0, i));
                            c += 1; // increment count

                            steps.push(step);
                            i += 1;
                        }
                        Err(_) => break
                    }
                }
                for (pc, (c, idx)) in pcs.drain().take(1) {
                    if c == 1 {
                        singles.insert(pc, idx);
                    }
                }

                let mut dump = (Dump{singles , steps});
                bincode::serialize_into(&mut out, &dump).unwrap();
                Ok(dump)
            }
        }
    }
}