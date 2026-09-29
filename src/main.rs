use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

fn candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = std::env::var_os("BURN_PATH") {
        out.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join(if cfg!(windows) { "burn.exe" } else { "burn" }));
        }
    }
    out.push(PathBuf::from(if cfg!(windows) {
        "burn.exe"
    } else {
        "burn"
    }));
    out
}

fn usage() {
    println!("burn-analyzer {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Starts the Burn language server over stdio by running `burn lsp`.");
    println!(
        "The burn executable is looked up in $BURN_PATH, next to burn-analyzer, and on $PATH."
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        usage();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("burn-analyzer {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    for burn in candidates() {
        let status = Command::new(&burn)
            .arg("lsp")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();
        match status {
            Ok(s) => return ExitCode::from(s.code().unwrap_or(1) as u8),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                eprintln!("burn-analyzer: could not start {}: {}", burn.display(), e);
                return ExitCode::from(1);
            }
        }
    }
    eprintln!(
        "burn-analyzer: the `burn` executable was not found. Install Burn 2 or set BURN_PATH."
    );
    ExitCode::from(1)
}
