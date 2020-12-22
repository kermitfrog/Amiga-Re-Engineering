use std::fs::File;
use std::io::{self, BufRead};
use serde::{Serialize, Deserialize};
use crate::utils::Colorizer;
use crate::memdump::MemDump;


big_array! { BigArray; }
#[derive(Serialize, Deserialize)]
pub struct CpuStep {
    pub data: [u32; 8],
    pub address: [u32; 8],
    pub usp: u32,
    pub isp: u32,
    pub sfc: u32,
    pub dfc: u32,
    pub cacr: u32,
    pub vbr: u32,
    pub caar: u32,
    pub msp: u32,
    pub t: u8,
    pub s: bool,
    pub m: bool,
    pub x: bool,
    pub n: bool,
    pub z: bool,
    pub v: bool,
    pub c: bool,
    pub imask: bool,
    pub stp: bool,
    pub pc: u32,
    pub pc_note: [u8; 24],
    #[serde(with = "BigArray")]
    pub note: [u8; 64],
    // 62
    pub pc_next: u32,
}

impl CpuStep {
    fn read_line(lines: &mut io::BufReader<File>, start_with: &str) -> Result<String, i8> {
        loop {
            let mut line = String::new();
            lines.read_line(&mut line).unwrap(); // potential crash acceptable... TODO better...
            if line.len() == 0 {
                return Err(0);
            }
            if line.starts_with(start_with) {
                return Ok(line);
            }
        }
    }

    fn set_registers(arr: &mut [u32; 8], line1: &str, line2: &str) {
        // assert_eq!(len(line), 56);
        let line = line1.get(0..line1.len() - 1).unwrap().to_owned() + line2;
        let mut offset = 0;
        for i in arr {
            let val = line.get(offset + 5..=offset + 12).unwrap_or_default();
            *i = u32::from_str_radix(val, 16).unwrap_or_default();
            offset += 14;
        }
    }

    pub fn from_dump(lines: &mut io::BufReader<File>) -> Result<CpuStep, i8> {
        let mut d: [u32; 8] = Default::default();
        let mut a: [u32; 8] = Default::default();
        // CpuStep::set_registers(&mut d, line_data1, line_data2);
        CpuStep::set_registers(&mut d,
                               CpuStep::read_line(lines, "  D0 ")?.as_str(),
                               CpuStep::read_line(lines, "  D4 ")?.as_str());
        CpuStep::set_registers(&mut a,
                               CpuStep::read_line(lines, "  A0 ")?.as_str(),
                               CpuStep::read_line(lines, "  A4 ")?.as_str());
        let l5s = CpuStep::read_line(lines, "USP  ")?;
        let line5 = l5s.as_str();
        let l6s = CpuStep::read_line(lines, "CACR ")?;
        let line6 = l6s.as_str();
        let lb = CpuStep::read_line(lines, "T=")?;
        let line_bits = lb.as_str();
        let lp = CpuStep::read_line(lines, "")?;
        let line_pc = lp.as_str();
        let lpn = CpuStep::read_line(lines, "Next PC")?;
        let line_next_pc = lpn.as_str();

        let pc_note = line_pc.get(9..=32).unwrap_or_default().as_bytes();
        let note = line_pc.get(34..line_pc.len()).unwrap_or_default().as_bytes();

        let step = CpuStep {
            data: d,
            address: a,
            usp: u32::from_str_radix(line5.get(5..=12).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            isp: u32::from_str_radix(line5.get(19..=26).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            sfc: u32::from_str_radix(line5.get(33..=40).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            dfc: u32::from_str_radix(line5.get(47..=54).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            cacr: u32::from_str_radix(line6.get(5..=12).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            vbr: u32::from_str_radix(line6.get(19..=26).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            caar: u32::from_str_radix(line6.get(33..=40).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            msp: u32::from_str_radix(line6.get(47..=54).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            t: u8::from_str_radix(line_bits.get(2..=3).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            s: line_bits.get(7..=7).unwrap_or("0") == "1",
            m: line_bits.get(11..=11).unwrap_or("0") == "1",
            x: line_bits.get(15..=15).unwrap_or("0") == "1",
            n: line_bits.get(19..=19).unwrap_or("0") == "1",
            z: line_bits.get(23..=23).unwrap_or("0") == "1",
            v: line_bits.get(27..=27).unwrap_or("0") == "1",
            c: line_bits.get(31..=31).unwrap_or("0") == "1",
            imask: line_bits.get(39..=39).unwrap_or("0") == "1",
            stp: line_bits.get(45..=45).unwrap_or("0") == "1",
            pc: u32::from_str_radix(line_pc.get(0..=7).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
            pc_note: array_init::array_init({
                |i| if i < pc_note.len() { pc_note[i] } else { 0x20 }
            }),
            note: array_init::array_init({
                |i| if i < note.len() - 1 { note[i] } else { 0x20 }
            }),
            pc_next: u32::from_str_radix(line_next_pc.get(9..=16).unwrap_or("0").as_ref(), 16).unwrap_or_default(),
        };

        Ok(step)
    }
    // returns register mask
    pub fn register_changed_to(&self, prev: &CpuStep, val: u32, mask: u32) -> u8 {
        let mut result: u8 = 0;
        let mut b: u8 = 1;
        for i in 0..=7 {
            if self.data[i] & mask == val && prev.data[i] & mask != val {
                result |= b;
            }
            b *= 2;
        }
        result
    }

    /// returns change in call depth by this instruction
    pub fn depth_mod(&self) -> i16 {
        /*
        +1 BSR, JSR
        -1 RTS, RTR
         */
        match self.note.get(0..3).unwrap_or_default() {
            [66, 83, 53] | [74, 83, 82] => 1,
            [82, 84, 83] | [82, 84, 82] => -1,
            _ => 0
        }
    }

    pub fn pretty_diff(&self, other: &CpuStep, mem: &MemDump, c: &Colorizer, num: usize) -> String {
        let mut s = String::new();

        // check data registers
        let mut print_newline = false;
        for i in 0..=7 {
            if self.data[i] != other.data[i] {
                print_newline = true;
                s += format!("D{} {}->{}  ", i,
                             c.col_reg(other.data[i]).as_str(),
                             c.col_reg(self.data[i]).as_str()
                ).as_str();
            }
        }
        if print_newline {
            s += "\n";
        }
        let note = std::str::from_utf8(&self.note).unwrap_or_default();
        let print_memory =
        match note.get(0..0).unwrap_or_default(){
            "A" | "D" | "O" => true,
            _ => {
                match note.get(0..=1).unwrap_or_default() {
                    "LS" | "RO" => true,
                    _ => {
                        match note.get(0..=2).unwrap_or_default() {
                            "CMP" | "EOR" | "MUL" | "NEG" | "NOT" | "SBC" | "SUB" => true,
                            _ => false
                        }
                    }
                }
            }
        };
        if print_memory {
            print_newline = false;
            for i in 2..self.note.len() - 1 {
                let x = self.note.get(i..=i+1).unwrap();
                match x {
                    [65, 48..=57] => {
                        print_newline = true;
                        let idx = (x[1] - 48) as usize;
                        let addr = self.address[idx];
                        s += format!("A{}: {}  ", idx, c.col(mem.get_mem_at(addr, 4))).as_str();
                    }
                    _ => {}
                }
            }
            if print_newline {
                s += "\n";
            }
        }
        s += format!("------ -{} \n\x1b[1m{:08X}\x1b[0m  {}\n", num, self.pc,
                     std::str::from_utf8(&self.note).unwrap_or_default()).as_str();
        s
    }
}

impl ToString for CpuStep {
    fn to_string(&self) -> String {
        format!("  D0 {:08x}   D1 {:08x}   D2 {:08x}   D3 {:08x}\
           \n  D4 {:08x}   D5 {:08x}   D6 {:08x}   D7 {:08x}\
           \n  A0 {:08x}   A1 {:08x}   A2 {:08x}   A3 {:08x}\
           \n  A4 {:08x}   A5 {:08x}   A6 {:08x}   A7 {:08x}\n\
           USP  {:08x} ISP  {:08x} SFC  {:08x} DFC  {:08x}\n\
           CACR {:08x} VBR  {:08x} CAAR {:08x} MSP  {:08x}\n\
           T={:02x} S={} M={} X={} N={} Z={} V={} C={} IMASK={} STP={}\n\
           {:08x} {:>24} {}\n\
           Next PC: {:08x}\
         ",
                self.data[0], self.data[1], self.data[2], self.data[3],
                self.data[4], self.data[5], self.data[6], self.data[7],
                self.address[0], self.address[1], self.address[2], self.address[3],
                self.address[4], self.address[5], self.address[6], self.address[7],
                self.usp, self.isp, self.sfc, self.dfc,
                self.cacr, self.vbr, self.caar, self.msp,
                self.t, self.s as u8, self.m as u8, self.x as u8, self.n as u8, self.z as u8,
                self.v as u8, self.c as u8, self.imask as u8, self.stp as u8,
                self.pc, std::str::from_utf8(&self.pc_note).unwrap_or_default(),
                std::str::from_utf8(&self.note).unwrap_or_default(),
                self.pc_next)
    }
}
