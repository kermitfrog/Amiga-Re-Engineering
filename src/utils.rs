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

use std::collections::BTreeSet;
use std::fs::{File};
use std::io::{BufReader, BufRead, Read};
use std::cmp::Ordering;
use clap::{ArgMatches, Values};
use crate::utils::Visibility::{Hidden, Brief};
use std::path::{PathBuf};
use roxmltree::{Document, ParsingOptions};
use std::iter::Peekable;

#[derive(Eq, PartialEq)]
pub enum Visibility { Hidden, Brief, Verbose }

/// stores variables for formatting the output
pub struct FormatHelper {
    /// replace occurrences of value1 with value2 - used for terminal colors / bold
    pub repl: Vec<(String, String)>,
    /// use compact (1 line per instruction) output?
    pub compact: bool,
    /// use colors?
    pub colors: bool,
    /// how much space to use to indent per step of the call hierarchy
    pub indent: i16,
    /// pc offset for disassembler
    pub offset_mod: u32,
    pub print_both_offsets: bool,
    pub func_names: Visibility,
    ///
    pub show_interrupt: Visibility,
    info: GhidraInfo,
}

#[derive(Eq)]
struct GhidraFun {
    pub start: u32,
    pub end: u32,
    pub name: String,
}

struct GhidraInfo {
    functions: BTreeSet<GhidraFun>,
    offset: u32,
}

impl FormatHelper {
    /// highlights values, by replacing them with colored versions
    pub fn col(&self, s: String) -> String {
        if !self.colors {
            return s;
        }
        let mut t = s.to_owned();
        // TODO this is buggy - need to work through list, then see if something can be found at
        // index. currently returning after first occurence, to prevent buggy output
        for (v, r) in &self.repl {
            if let Some(i) = s.find(v) {
                if i % 2 == 0 {
                    t.replace_range((t.len() - v.len())..t.len(), r.as_str());
                    return t.to_string();
                }
            }
        }
        t.to_string()
    }

    /// return val as hex string, highlighting values, by replacing them with colored versions
    pub fn col_reg(&self, val: u32) -> String {
        let mut s = format!("{:08X}", val);
        for (v, r) in &self.repl {
            if s.ends_with(&v.as_str()) {
                s.replace_range((8 - v.len())..8, r.as_str());
            }
        }
        s
    }

    /// construct FormatHelper with highlighting for certain values
    ///
    /// highlight: values to be colored
    pub fn for_values(highlight: &Values, compact: bool) -> FormatHelper {
        // pub fn for_values(highlight: &Vec<String>, compact: bool) -> FormatHelper {
        // prepare colors
        let colors = [
            "\x1b[32m", // green
            "\x1b[35m", // magenta
            "\x1b[36m", // cyan
            "\x1b[34m", // blue
            "\x1b[31m", // red
            "\x1b[33m", // yellow
        ];
        let mut replacements: Vec<(String, String)> = Vec::new();
        replacements.reserve(highlight.len());
        for (h, i) in highlight.to_owned().zip(0..) {
            // for (h, i) in highlight.iter().zip(0..) {
            replacements.push((h.to_string(), format!("{}{}\x1b[0m",
                                                      colors.get(i % colors.len()).unwrap(), h)));
        }
        FormatHelper {
            repl: replacements,
            compact,
            colors: true,
            indent: 2,
            offset_mod: 0,
            print_both_offsets: true,
            func_names: Visibility::Verbose,
            show_interrupt: Visibility::Brief,
            info: GhidraInfo { functions: BTreeSet::new(), offset: 0 },
        }
    }

    /// construct FormatHelper without highlighting
    pub fn simple(compact: bool) -> FormatHelper {
        FormatHelper {
            repl: Vec::new(),
            compact,
            colors: false,
            indent: 2,
            offset_mod: 0,
            print_both_offsets: true,
            func_names: Visibility::Brief,
            show_interrupt: Visibility::Brief,
            info: GhidraInfo { functions: BTreeSet::new(), offset: 0 },
        }
    }

    pub fn finalize(mut self, args: &ArgMatches) -> FormatHelper {
        if args.value_of("colors").is_some() {
            self.colors = true;
        } else if args.value_of("nocolors").is_some() {
            self.colors = false;
        }

        if let Some(indent) = args.value_of("indent") {
            self.indent = i16::from_str_radix(indent, 10).unwrap_or(2);
        }

        match args.value_of("offset-mode") {
            Some("dump") => {
                self.offset_mod = 0;
                self.print_both_offsets = false;
            }
            Some("translated") => {
                self.offset_mod = FormatHelper::get_offset(&args);
                self.print_both_offsets = false
            }
            Some("both") => {
                self.offset_mod = FormatHelper::get_offset(&args);
                self.print_both_offsets = true
            }
            _ => {
                if self.print_both_offsets {
                    self.offset_mod = FormatHelper::get_offset(&args);
                }
            }
        }

        match args.value_of("function-names") {
            Some("never") => self.func_names = Hidden,
            Some("entry") => self.func_names = Brief,
            Some("always") => self.func_names = Visibility::Verbose,
            _ => {}
        }

        if self.func_names != Hidden {
            self.info.load(args, self.offset_mod);
        }

        if args.is_present("traps") {
            self.show_interrupt = Brief;
        }

        // todo load ghidra info

        return self;
    }

    /// load offset from path/offset or 0 if file is missing
    pub fn get_offset(args: &ArgMatches) -> u32 {
        let file_offset = FormatHelper::file_in_dir_or_parent(&args, "offset");
        match file_offset {
            Some(file) => {
                let mut buf_reader = BufReader::new(file);
                let mut s = String::new();
                let _ = buf_reader.read_line(&mut s);
                u32::from_str_radix(&s.trim_end(), 16).unwrap_or_default()
            }
            None => 0u32
        }
    }

    pub fn with_offset(&self, address: u32) -> u32 {
        return if address >= self.offset_mod {
            address - self.offset_mod
        } else {
            address + 0xf0000000
        };
    }

    pub fn pc(&self, pc: u32) -> String {
        match (&self.func_names, &self.print_both_offsets) {
            (Hidden, false) => format!("{:08X}", self.with_offset(pc)),
            (Hidden, true) => format!("{:08X} ({:08X})", self.with_offset(pc), pc),
            (_, false) => {
                if let Some(s) = self.info.name_for(pc) {
                    format!("{} ({:08X})", s, self.with_offset(pc))
                } else {
                    format!("{:08X}", self.with_offset(pc))
                }
            }
            (_, true) => {
                if let Some(s) = self.info.name_for(pc) {
                    format!("{} ({:08X}, {:08X})", s, self.with_offset(pc), pc)
                } else {
                    format!("{:08X} ({:08X})", self.with_offset(pc), pc)
                }
            }
        }
    }

    pub fn padding(&self, depth: i16) -> String {
        let pad_max: usize = 60;
        let pad: usize = if depth >= 0 { (depth * self.indent) as usize } else { 0 };
        // let pad_inline = if compact {0i16} else { pad };
        return if pad <= pad_max {
            format!("{:>width$}", "", width = pad)
        } else {
            format!("{:>width$}  ", depth, width = pad_max - 2)
        };
    }

    pub fn file_in_dir_or_parent(args: &ArgMatches, f_name: &str) -> Option<File> {
        let mut p: Peekable<Values>;
        let mut path = PathBuf::from(
            if let Some(d) = args.value_of("dir") {
                d
            } else if let Some(d) = args.value_of("set_dir") {
                d
            } else if let Some(d) = args.values_of("dir val")
            {
                if d.len() < 2 {
                    return None;
                }
                p = d.peekable();
                p.peek().unwrap()
            } else {
                return None;
            });
        path.push(f_name);
        if !path.exists() {
            path.pop();
            path.pop();
            path.push(f_name);
        }
        if let Ok(file) = File::open(path) {
            Some(file)
        } else {
            None
        }
    }
}

impl GhidraInfo {
    pub fn load(&mut self, args: &ArgMatches, offset: u32) {
        self.offset = offset;
        if let Some(mut file) = FormatHelper::file_in_dir_or_parent(&args, "functions.xml") {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_err() {
                return;
            }
            match Document::parse_with_options(content.as_str(), ParsingOptions { allow_dtd: true }) {
                Ok(xml) =>
                    if let Some(funs) = xml.descendants()
                        .find(|&n| n.has_tag_name("FUNCTIONS")) {
                        for fun in funs.children() {
                            let mut gf = GhidraFun::new();
                            if let Some(name) = fun.attribute("NAME") {
                                gf.name = name.into();
                            } else { continue; }

                            if let Some(addresses) = fun.children()
                                .find(|&n| n.has_tag_name("ADDRESS_RANGE")) {
                                if let Some(val) = addresses.attribute("START") {
                                    gf.start = u32::from_str_radix(val, 16).unwrap_or(0)
                                } else { continue; }

                                if let Some(val) = addresses.attribute("END") {
                                    gf.end = u32::from_str_radix(val, 16).unwrap_or(0)
                                } else { continue; }
                            } else { continue; }

                            if gf.start != 0 && gf.end != 0 {
                                self.functions.insert(gf);
                            }
                        }
                    },
                Err(e) => println!("Error loading functions.xml: {}", e)
            }
        }
    }

    pub fn name_for(&self, address: u32) -> Option<String> {
        let pc = address.wrapping_sub(self.offset);
        let tmp = GhidraFun { start: pc, end: pc, name: String::new() };
        if let Some(closest) = self.functions.range(..=tmp).next_back() {
            if (closest.start..=closest.end).contains(&pc) {
                return Some(closest.name.to_owned());
            }
        }
        None
    }
}

impl GhidraFun {
    pub fn new() -> GhidraFun {
        GhidraFun { start: 0, end: 0, name: String::new() }
    }
}

impl Ord for GhidraFun {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}

impl PartialOrd for GhidraFun {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for GhidraFun {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start
    }
}