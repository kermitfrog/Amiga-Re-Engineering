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
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::cmp::{min, max};

pub struct MemDump {
    /// structure for a partial memory dump
    parts: Vec<MemPart>
}

struct MemPart {
    /// structure for a consecutive part of MemDump
    from: u32,
    to: u32,
    data: Vec<u8>,
}

impl MemDump {
    /// create empty MemDump
    pub fn new() -> MemDump {MemDump{parts: Vec::new()}}

    /// load MemDump from a file called mem in directory path
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

    /// returns count bytes from MemDump at address addr, or "??" if addr is not in current dump
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