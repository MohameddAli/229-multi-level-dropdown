use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match fs::metadata(path) {
        Ok(metadata) => {
            let permissions = metadata.permissions();
            metadata.is_file() && (permissions.mode() & 0o111 != 0)
        }
        Err(_) => false,
    }
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    path.is_file()
        && path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("exe"))
}

fn main() {
    let builtins: HashSet<&str> = ["exit", "echo", "type"].iter().cloned().collect();

    loop {
        print!("$ ");
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }

        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let command = parts[0];

        match command {
            "exit" => {
                let exit_code = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                process::exit(exit_code);
            }
            "echo" => {
                let output = parts[1..].join(" ");
                println!("{}", output);
            }
            "type" => {
                if parts.len() < 2 {
                    continue;
                }
                let cmd_to_check = parts[1];

                // الخطوة 1: التحقق من الأوامر المدمجة أولاً
                if builtins.contains(cmd_to_check) {
                    println!("{} is a shell builtin", cmd_to_check);
                    continue;
                }

                // الخطوة 2: البحث في مجلدات PATH
                let path_var = env::var("PATH").unwrap_or_default();
                let dirs = path_var.split(':').collect::<Vec<&str>>();
                let mut found = false;

                for dir in dirs {
                    let full_path = Path::new(dir).join(cmd_to_check);
                    if is_executable(&full_path) {
                        println!("{} is {}", cmd_to_check, full_path.display());
                        found = true;
                        break;
                    }
                }

                if !found {
                    println!("{}: not found", cmd_to_check);
                }
            }
            _ => println!("{}: command not found", command),
        }
    }
}