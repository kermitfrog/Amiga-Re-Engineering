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
use std::io::{self, BufRead};
use serde::{Serialize, Deserialize};
use crate::utils::FormatHelper;
use crate::memdump::MemDump;

// use BigArray, as this is needed to allow serde to handle arrays beyond 32 elements
big_array! { BigArray; }
#[derive(Serialize, Deserialize)]
pub struct CpuStep {
    /// an instruction step, containing the info about register state from fs-uae. note is the
    /// instruction as disassembled by fs-uae.
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
    /// helper function to read a line from the dump, trying to skip output, that is not part of the
    /// data we are looking for.
    ///
    /// lines: BufReader with position at the beginning of the next line
    /// start_with: expected condition. If the line does not start with this String, skip lines
    /// until we find one that does.
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

    /// helper function to parse registers (D0-D7 or A0-A7) into an array
    ///
    /// arr: array to write into
    /// line1 first line (containing D0-D3 or A0-A3)
    /// line2 second line (containing D4-D7 or A4-A7)
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

    /// reads one instruction step and the register contents from the dump
    ///
    /// lines: BufReader with position at the beginning of the instruction step
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
    /// returns u8 with bits signifying which data registers have changed their value to val
    ///
    /// prev: instruction to compare with
    /// val: value, we're looking for
    /// mask: bit mask for value (e.g. if we're only interested it 16 bit values)
    pub fn register_changed_to(&self, prev: &CpuStep, val: u32, mask: u32) -> u8 {
        let mut result: u8 = 0;
        let mut b: u8 = 1;
        for i in 0..=7 {
            if self.data[i] & mask == val && prev.data[i] & mask != val {
                result |= b;
            }
            // Multiplication with intended (well, here: don't care) overflow.
            // Otherwise, Rust will panic in debug mode (not in release)
            b = b.wrapping_mul(2);
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

    /// Generate String showing the difference between 2 instruction steps
    ///
    /// other: instruction steps to compare with
    /// mem: Memory dump (for printing possible content, an address register is pointing at)
    /// fmt: formatting configuration
    /// num: number of steps until end pc is reached. Only printed at depth change.
    /// depth: current call depth. Used for padding and modified on change.
    pub fn pretty_diff(&self, other: &CpuStep, mem: &MemDump, fmt: &FormatHelper, num: usize, depth: &mut i16) -> String {
        let mut s = String::new();
        let pad: usize = if *depth >= 0 {(*depth * fmt.indent) as usize} else {0};
        // let pad_inline = if compact {0i16} else { pad };
        let padding = format!("{:>width$}", "", width = pad);
        let mut delimiter = String::new();
        if fmt.compact {delimiter += "  "} else {
            let nl = format!("\n{}", &padding);
            delimiter += nl.as_str();
        };

        // check data registers
        let mut print_spacing = false;
        for i in 0..=7 {
            if self.data[i] != other.data[i] {
                print_spacing = true;
                s += format!("D{} {}->{}  ", i,
                             fmt.col_reg(other.data[i]).as_str(),
                             fmt.col_reg(self.data[i]).as_str()
                ).as_str();
            }
        }
        if print_spacing {
            s += delimiter.as_str();
        }
        // address registers are only parsed for certain instructions
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
        if print_memory { // TODO does not always work correctly
            print_spacing = false;
            for i in 2..self.note.len() - 1 {
                let x = self.note.get(i..=i+1).unwrap();
                match x {
                    [65, 48..=57] => {
                        print_spacing = true;
                        let idx = (x[1] - 48) as usize;
                        let addr = self.address[idx];
                        s += format!("A{}: {}  ", idx, fmt.col(mem.get_mem_at(addr, 4))).as_str();
                    }
                    _ => {}
                }
            }
            if print_spacing {
                s += delimiter.as_str();
            }
        }
        let depth_m = self.depth_mod();
        if depth_m > 0 || other.depth_mod() < 0 {
            s += format!("\n{}##-{:<5}", padding, num).as_str();
        }
        *depth += depth_m;
        if fmt.compact {
            s += format!("\n{}{:08X}  {}", padding, self.pc - fmt.offset_mod,
                         std::str::from_utf8(&self.note).unwrap_or_default()).as_str();
        } else {
            s += format!("\n{}\x1b[1m{:08X}\x1b[0m  {}{}", padding, self.pc - fmt.offset_mod,
                         std::str::from_utf8(&self.note).unwrap_or_default(), delimiter).as_str()
        }
        s
    }

    /// print instruction in format, suitable for ghidra's instruction search feature
    pub fn print_for_search(&self, prev: &CpuStep) -> Result<String, &str> {
        let mut diff = (prev.pc_next - self.pc) as i32;
        return if diff == 0 {
            Ok(std::str::from_utf8(&self.pc_note).unwrap_or_default().trim_end().to_string())
        } else if diff < 0 {
            Err("Negative step not implemented")
        } else if diff > 80 {
            Err("Big step -- aborting now")
        } else {
            let mut s = String::new();
            while diff > 0 {
                s += "[........] ";
                diff -= 1;
            }
            Ok(s.trim_end().to_string())
        }
    }
}

impl ToString for CpuStep {
    /// output as from fs-uae
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
