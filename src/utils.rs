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

pub enum Visibility { Hidden, Note }//, Verbose }

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
    ///
    pub show_interrupt: Visibility,
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
    pub fn for_values(highlight: &Vec<String>, compact: bool, indent: i16, offset_mod: u32) -> FormatHelper {
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
        for (h, i) in highlight.iter().zip(0..) {
            replacements.push((h.to_string(), format!("{}{}\x1b[0m",
                                                      colors.get(i % colors.len()).unwrap(), h)));
        }
        FormatHelper { repl: replacements, compact, colors: true, indent, offset_mod,
            show_interrupt: Visibility::Note }
    }

    /// construct FormatHelper without highlighting
    pub fn simple(compact: bool, indent: i16, offset_mod: u32) -> FormatHelper {
        FormatHelper { repl: Vec::new(), compact, colors: false, indent, offset_mod,
            show_interrupt: Visibility::Note }
    }

    pub fn with_offset(&self, address: u32) -> u32 {
        return if address >= self.offset_mod {
            address - self.offset_mod
        } else {
            address + 0xf0000000
        };
    }

    pub fn string(&self, address: u32) -> String {
        return String::new();
    }

    pub fn padding(&self, depth: i16) -> String {
        let pad_max: usize = 20;
        let pad: usize = if depth >= 0 { (depth * self.indent) as usize } else { 0 };
        // let pad_inline = if compact {0i16} else { pad };
        return if pad <= pad_max {
            format!("{:>width$}", "", width = pad)
        } else {
            format!("{:>width$}  ", depth, width = pad_max - 2)
        };
    }
}