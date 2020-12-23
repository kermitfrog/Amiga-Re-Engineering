
pub struct FormatHelper {
    pub repl: Vec<(String, String)>,
    pub compact: bool,
    pub indent: i8
}

impl FormatHelper {
    pub fn col(&self, s: String) -> String {
        let mut t = s.to_owned();
        // TODO this is buggy - need to work through list, then see if something can be found at
        // index. currently returning after first occurence, to prevent buggy output
        for (v, r) in &self.repl {
            if let Some(i) = s.find(v) {
                if i % 2 == 0 {
                    t.replace_range((t.len()-v.len())..t.len(), r.as_str());
                    return t.to_string();
                }
            }
        }
        t.to_string()
    }

    pub fn col_reg(&self, val: u32) -> String {
        let mut s = format!("{:08X}", val);
        for (v, r) in &self.repl {
            if s.ends_with(&v.as_str()) {
                s.replace_range((8-v.len())..8, r.as_str());
            }
        }
        s
    }

    pub fn for_values(highlight: &Vec<String>, compact: bool, indent: i8) -> FormatHelper {
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
        FormatHelper {repl: replacements, compact, indent }
    }
}