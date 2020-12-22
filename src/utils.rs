
pub struct Colorizer {
    pub repl: Vec<(String, String)>
}

impl Colorizer {
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
}