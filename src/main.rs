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

fn find_on_path(p: &std::path::Path) -> Option<PathBuf> {
    if p.components().count() > 1 {
        return Some(p.to_path_buf());
    }
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|d| d.join(p))
        .find(|c| c.is_file())
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
    if std::env::var_os("BURN_ANALYZER_ACTIVE").is_some() {
        eprintln!("burn-analyzer: burn-analyzer was started by itself; set BURN_PATH to the burn executable, not to burn-analyzer");
        return ExitCode::from(1);
    }
    let me = std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::canonicalize(p).ok());
    for burn in candidates() {
        let resolved = find_on_path(&burn).and_then(|p| std::fs::canonicalize(p).ok());
        if resolved.is_some() && resolved == me {
            continue;
        }
        let status = Command::new(&burn)
            .arg("lsp")
            .env("BURN_ANALYZER_ACTIVE", "1")
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
    eprintln!("burn-analyzer: the `burn` executable was not found. Install Burn or set BURN_PATH.");
    ExitCode::from(1)
}
