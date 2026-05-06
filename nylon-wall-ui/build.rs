//! Build script that compiles `tailwind.css` (Tailwind v4 source) into
//! `assets/tailwind.css` so the `asset!()` macro can find it during `cargo
//! check`/`cargo build`. `dx build` performs its own pass and overwrites this
//! output, which is fine — both produce the same content.
//!
//! Strategy:
//!   1. If `assets/tailwind.css` already exists and is newer than the source,
//!      do nothing.
//!   2. Otherwise, look for a `tailwindcss` binary (system $PATH or the one
//!      cached by `dx`) and run it.
//!   3. If we can't find a binary, write a placeholder so the asset macro
//!      compiles. Real builds run via `dx build`, which will fill it in.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

fn main() {
    println!("cargo:rerun-if-changed=tailwind.css");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=index.html");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);
    let input = manifest_dir.join("tailwind.css");
    let assets_dir = manifest_dir.join("assets");
    let output = assets_dir.join("tailwind.css");

    if !input.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&assets_dir);

    if output_is_fresh(&input, &output) {
        return;
    }

    if let Some(bin) = find_tailwind_binary() {
        let status = Command::new(&bin)
            .args(["-i", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
            .current_dir(manifest_dir)
            .status();
        match status {
            Ok(s) if s.success() => return,
            _ => println!("cargo:warning=tailwindcss compilation failed; using placeholder"),
        }
    } else {
        println!(
            "cargo:warning=tailwindcss CLI not found; writing placeholder assets/tailwind.css. \
             Run `dx build` for the real compilation."
        );
    }

    if !output.exists() {
        let _ = std::fs::write(
            &output,
            "/* placeholder - run `dx build` to generate Tailwind output */\n",
        );
    }
}

fn output_is_fresh(input: &Path, output: &Path) -> bool {
    let (Ok(in_meta), Ok(out_meta)) = (input.metadata(), output.metadata()) else {
        return false;
    };
    let in_time = in_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let out_time = out_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    out_time >= in_time && out_meta.len() > 100
}

fn find_tailwind_binary() -> Option<PathBuf> {
    // 1. Explicit override
    if let Ok(p) = std::env::var("TAILWINDCSS_BIN") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }

    // 2. dx's cached standalone binary
    if let Some(home) = std::env::var_os("HOME") {
        let dx_dir = PathBuf::from(home).join(".local/share/.dx/tailwind");
        if let Ok(entries) = std::fs::read_dir(&dx_dir) {
            // Pick the highest-numbered version available.
            let mut found: Option<PathBuf> = None;
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    found = match (found, p.file_name().map(|f| f.to_owned())) {
                        (None, _) => Some(p),
                        (Some(prev), Some(name)) if name > prev.file_name().unwrap().to_owned() => {
                            Some(p)
                        }
                        (prev, _) => prev,
                    };
                }
            }
            if let Some(p) = found {
                return Some(p);
            }
        }
    }

    // 3. Tools on PATH
    for name in ["tailwindcss", "npx"] {
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }

    None
}
