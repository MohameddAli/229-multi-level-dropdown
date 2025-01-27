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
        env::split_paths(&path)
            .filter_map(|dir| {
                let full_path = dir.join(command);
                if is_executable(&full_path) {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next()
    })
}

fn parse_arguments(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut pos = 0;

    #[derive(PartialEq)]
    enum QuoteState {
        None,
        Single,
        Double,
    }

    while pos < len {
        // Skip whitespace
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }
        if pos >= len {
            break;
        }

        let mut buffer = String::new();
        let mut quote_state = QuoteState::None;

        while pos < len {
            let c = chars[pos];
            match quote_state {
                QuoteState::None => {
                    if c == '\'' {
                        quote_state = QuoteState::Single;
                        pos += 1;
                    } else if c == '"' {
                        quote_state = QuoteState::Double;
                        pos += 1;
                    } else if c == '\\' {
                        // Handle backslash escape outside quotes
                        pos += 1; // Skip the backslash
                        if pos < len {
                            buffer.push(chars[pos]);
                            pos += 1;
                        } else {
                            // Backslash at the end of input, add it
                            buffer.push('\\');
                        }
                    } else if c.is_whitespace() {
                        break;
                    } else {
                        buffer.push(c);
                        pos += 1;
                    }
                }
                QuoteState::Single => {
                    if c == '\'' {
                        quote_state = QuoteState::None;
                        pos += 1;
                    } else {
                        buffer.push(c);
                        pos += 1;
                    }
                }
                QuoteState::Double => {
                    if c == '\\' {
                        pos += 1; // Skip the backslash
                        if pos < len {
                            let next_c = chars[pos];
                            if matches!(next_c, '\\' | '$' | '"' | '\n') {
                                buffer.push(next_c);
                                pos += 1;
                            } else {
                                // Add the backslash and the next character
                                buffer.push('\\');
                                buffer.push(next_c);
                                pos += 1;
                            }
                        } else {
                            // Backslash at the end, add it
                            buffer.push('\\');
                        }
                    } else if c == '"' {
                        quote_state = QuoteState::None;
                        pos += 1;
                    } else {
                        buffer.push(c);
                        pos += 1;
                    }
                }
            }
        }

        tokens.push(buffer);
    }

    tokens
}

fn main() -> io::Result<()> {
    let builtins: HashSet<&str> = ["exit", "echo", "type", "pwd", "cd"].iter().cloned().collect();

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

        let parts = parse_arguments(trimmed);
        if parts.is_empty() {
            continue;
        }
        let command = &parts[0];

        match command.as_str() {
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
                let cmd_to_check = &parts[1];
                if builtins.contains(cmd_to_check.as_str()) {
                    println!("{} is a shell builtin", cmd_to_check);
                    continue;
                }

                if let Some(path) = find_in_path(cmd_to_check) {
                    println!("{} is {}", cmd_to_check, path.display());
                } else {
                    println!("{}: not found", cmd_to_check);
                }
            }
            "pwd" => {
                let current_dir = env::current_dir()?;
                println!("{}", current_dir.display());
            }
            "cd" => {
                if parts.len() != 2 {
                    eprintln!("cd: expected 1 argument, got {}", parts.len() - 1);
                    continue;
                }
                let new_dir = &parts[1];
                let path = if new_dir == "~" {
                    match env::var_os("HOME") {
                        Some(home) => PathBuf::from(home),
                        None => {
                            eprintln!("cd: HOME environment variable not set");
                            continue;
                        }
                    }
                } else {
                    PathBuf::from(new_dir)
                };
                match env::set_current_dir(&path) {
                    Ok(()) => {}
                    Err(e) => {
                        if e.kind() == io::ErrorKind::NotFound {
                            eprintln!("cd: {}: No such file or directory", path.display());
                        } else {
                            eprintln!("cd: {}", e);
                        }
                    }
                }
            }
            _ => {
                if builtins.contains(command.as_str()) {
                    println!("{}: command not found", command);
                    continue;
                }

                let program_path = if let Some(path) = find_in_path(command) {
                    path
                } else {
                    println!("{}: command not found", command);
                    continue;
                };

                let args = parts
                    .iter()
                    .skip(1)
                    .map(|s| OsStr::new(s.as_str()))
                    .collect::<Vec<_>>();

                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    let mut cmd = Command::new(&program_path);
                    if let Some(file_name) = program_path.file_name().and_then(|n| n.to_str()) {
                        cmd.arg0(file_name);
                    }
                    let status = cmd
                        .args(&args)
                        .status()
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    if !status.success() {
                        eprintln!("Process exited with code: {:?}", status.code());
                    }
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