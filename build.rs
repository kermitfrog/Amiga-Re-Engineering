use clap_generate::generate_to;
use clap_generate::generators::*;
use std::fs;

include!("src/cli.rs");

fn main() {
    let mut app = args();
    app.set_bin_name("dump-analyzer");

    let outdir = env!("CARGO_MANIFEST_DIR").to_owned() + "/shell/";
    fs::create_dir(&outdir).unwrap_or_default();
    generate_to::<Zsh, _, _>(&mut app, "dump-analyzer", &outdir);
    generate_to::<Bash, _, _>(&mut app, "dump-analyzer", &outdir);
    generate_to::<Elvish, _, _>(&mut app, "dump-analyzer", &outdir);
    generate_to::<PowerShell, _, _>(&mut app, "dump-analyzer", &outdir);
    generate_to::<Fish, _, _>(&mut app, "dump-analyzer", &outdir);
}