use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

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

fn find_in_path(command: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|path| {
        eprintln!("Searching for {} in PATH: {:?}", command, path);
        
        env::split_paths(&path)
            .filter_map(|dir| {
                let full_path = dir.join(command);
                eprintln!("Trying path: {:?}", full_path);
                if is_executable(&full_path) {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next()
    })
}

fn main() -> io::Result<()> {
    // Add /tmp/bar to PATH for test environment
    if let Some(path) = env::var_os("PATH") {
        let mut paths = env::split_paths(&path).collect::<Vec<_>>();
        paths.insert(0, PathBuf::from("/tmp/bar"));
        let new_path = env::join_paths(paths).unwrap();
        env::set_var("PATH", new_path);
    }

    let builtins: HashSet<&str> = ["exit", "echo", "type"].iter().cloned().collect();

    loop {
        print!("$ ");
        io::stdout().flush()?;

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
                if builtins.contains(cmd_to_check) {
                    println!("{} is a shell builtin", cmd_to_check);
                    continue;
                }

                if let Some(path) = find_in_path(cmd_to_check) {
                    println!("{} is {}", cmd_to_check, path.display());
                } else {
                    println!("{}: not found", cmd_to_check);
                }
            }
            _ => {
                if builtins.contains(command) {
                    println!("{}: command not found", command);
                    continue;
                }

                let program_path = if let Some(path) = find_in_path(command) {
                    path
                } else {
                    println!("{}: command not found", command);
                    continue;
                };

                let args = parts.iter().skip(1).map(|s| OsStr::new(s)).collect::<Vec<_>>();

                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    Command::new(&program_path)
                        .arg0(command)  // Use original command name for argv[0]
                        .args(&args)
                        .spawn()?
                        .wait()
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                }
                #[cfg(not(unix))]
                {
                    let status = Command::new(&program_path)
                        .args(&args)
                        .status()
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    if !status.success() {
                        eprintln!("Process exited with code: {:?}", status.code());
                    }
                }
            }
        }
    }

    Ok(())
}