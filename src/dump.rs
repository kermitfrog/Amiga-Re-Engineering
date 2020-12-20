use crate::cpustep::CpuStep;
use std::collections::{HashMap, BTreeMap};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Dump {
    // name: str,
    singles: HashMap<u32, usize>, // PC -> Offset
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

                let mut pc_last : u32 = 0;
                let mut pc_new : u32;
                for (pc, idx) in &singles_all {
                    pc_new = *pc;
                    // if pc_new > pc_last + 10 {
                    if pc_new == steps.get(singles_all[&pc_last]).unwrap().pc_next {
                        singles.insert(*pc, *idx);
                        println!("({:x}, {})", pc, idx);
                    }
                    pc_last = pc_new;
                }


                println!("pre create bin");
                let dump = Dump{singles , steps};
                let out = File::create(path.to_owned() + "/opcode.bin")?;
                let mut out_buf = BufWriter::new(out);
                bincode::serialize_into(&mut out_buf, &dump).unwrap();
                println!("post create bin");
                Ok(dump)
            }
        }
    }
}