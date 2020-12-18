use std::fs::File;
use std::io::{self, BufRead};
use arraystring::{ArrayString, typenum::U24, typenum::U80};

pub struct CpuStep {
    data: [u32; 8],
    address: [u32; 8],
    usp: u32,
    isp: u32,
    sfc: u32,
    dfc: u32,
    cacr: u32,
    vbr: u32,
    caar: u32,
    msp: u32,
    t: u8,
    s: bool,
    m: bool,
    x: bool,
    n: bool,
    z: bool,
    v: bool,
    c: bool,
    imask: bool,
    stp: bool,
    pc: u32,
    pc_note: ArrayString<U24>,
    note: ArrayString<U80>,
    pc_next: u32
}

impl CpuStep {
    fn read_line(lines: &mut io::BufReader<File>, start_with: &str) -> std::io::Result<String> {
        loop {
            let mut line = String::new();
            lines.read_line(&mut line)?;
            if line.starts_with(start_with) {
                return Ok(line);
            }
        }
    }

    fn set_registers(arr: &mut [u32; 8], line1: &str, line2: &str) {
        // assert_eq!(len(line), 56);
        let line = line1.get(0..line1.len()-1).unwrap().to_owned() + line2;
        let mut offset = 0;
        for i in arr {
            let val = line.get(offset + 5 ..= offset + 12).unwrap_or_default();
            *i = u32::from_str_radix(val, 16).unwrap_or_default();
            offset += 14;
        }
    }

    pub fn from_dump(lines: &mut io::BufReader<File>) -> std::io::Result<CpuStep> {
        let mut d : [u32; 8] = Default::default();
        let mut a : [u32; 8] = Default::default();
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
            pc_note: line_pc.get(8..=32).unwrap_or_default().into(),
            note: line_pc.get(35..=line_pc.len() - 1).unwrap_or_default().into(),
            pc_next: u32::from_str_radix(line_next_pc.get(9..=16).unwrap_or("0").as_ref(), 16).unwrap_or_default()
        };

        Ok(step)
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
           {:08x} {:>24} {}\
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
            self.pc, &self.pc_note, &self.note,
            self.pc_next)
    }
}