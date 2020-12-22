use std::fs::File;
use std::io::{BufReader, BufRead};
use std::cmp::{min, max};

pub struct MemDump {
    parts: Vec<MemPart>
}

struct MemPart {
    from: u32,
    to: u32,
    data: Vec<u8>,
}

impl MemDump {
    pub fn new() -> MemDump {MemDump{parts: Vec::new()}}

    /// expects fs-uae memdump from debugger, '>' removed, newline at end
    pub fn from_dir(path: String) -> std::io::Result<MemDump> {
        let file = File::open(path.to_owned() + "/mem")?;
        let mut buf_reader = BufReader::new(file);
        let mut part = MemPart{from: 0, to: 0, data: Vec::new()};
        let mut mem_dump = MemDump{parts: Vec::new()};

        loop {
            let mut line = Default::default();
            let n = buf_reader.read_line(&mut line)?;
            if n <= 48 {
                break;
            }
            let addr = u32::from_str_radix(line.get(0..=7).unwrap(), 16).unwrap();
            if part.from == 0 {
                part.from = addr;
            } else if part.to + 16 < addr {
                mem_dump.parts.push(part);
                part = MemPart {from: addr, to: addr + 15, data: Vec::new()};
            }
            part.to = addr + 15;
            let mut offset = 0;
            let mut gap = false;
            for _ in 0..16 {
                let val = line.get(offset + 9..=offset + 10).unwrap_or_default();
                part.data.push(u8::from_str_radix(val, 16).unwrap_or_default());
                offset += if gap {3} else {2};
                gap = !gap;
            }
        }
        mem_dump.parts.push(part);
        Ok(mem_dump)
    }

    pub fn get_mem_at(&self, addr: u32, count: usize) -> String {
        for part in &self.parts {
            if (part.from..=part.to).contains(&addr) {
                let from = (addr & 0xffffffe - part.from) as usize;
                let to = min( from + max(count, 4), part.to as usize);
                let mut r = format!("{:08X}= ", addr);
                for i in 0..(to-from) {
                    r += format!("{:02x}", part.data[i]).as_str();
                }
                return r
            }
        }
        format!("{:08X}: ??", addr)
    }
}