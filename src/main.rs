use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
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
                        pos += 1;
                        if pos < len {
                            buffer.push(chars[pos]);
                            pos += 1;
                        } else {
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
                        pos += 1;
                        if pos < len {
                            let next_c = chars[pos];
                            if matches!(next_c, '\\' | '$' | '"' | '\n') {
                                buffer.push(next_c);
                                pos += 1;
                            } else {
                                buffer.push('\\');
                                buffer.push(next_c);
                                pos += 1;
                            }
                        } else {
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

fn split_redirects(token: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let chars: Vec<char> = token.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if i + 2 < len && chars[i] == '1' && chars[i + 1] == '>' && chars[i + 2] == '>' {
            parts.push("1>>".to_string());
            i += 3;
        } else if i + 1 < len && chars[i] == '1' && chars[i + 1] == '>' {
            parts.push("1>".to_string());
            i += 2;
        } else if i + 2 < len && chars[i] == '2' && chars[i + 1] == '>' && chars[i + 2] == '>' {
            parts.push("2>>".to_string());
            i += 3;
        } else if i + 1 < len && chars[i] == '2' && chars[i + 1] == '>' {
            parts.push("2>".to_string());
            i += 2;
        } else if i + 1 < len && chars[i] == '>' && chars[i + 1] == '>' {
            parts.push(">>".to_string());
            i += 2;
        } else if chars[i] == '>' {
            parts.push(">".to_string());
            i += 1;
        } else {
            let mut current = String::new();
            while i < len
                && !(chars[i] == '>'
                    || chars[i] == '1'
                    || chars[i] == '2'
                    || (i + 1 < len && chars[i] == '>' && chars[i + 1] == '>'))
            {
                current.push(chars[i]);
                i += 1;
            }
            if !current.is_empty() {
                parts.push(current);
            }
        }
    }

    parts
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
        let parts = parts
            .into_iter()
            .flat_map(|token| split_redirects(&token))
            .collect::<Vec<_>>();

        let mut command_args = Vec::new();
        let mut stdout_redirect = None; // (path, append)
        let mut stderr_redirect = None; // (path, append)

        let mut i = 0;
        while i < parts.len() {
            match parts[i].as_str() {
                ">" | "1>" => {
                    if i + 1 < parts.len() {
                        stdout_redirect = Some((parts[i + 1].clone(), false));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                ">>" | "1>>" => {
                    if i + 1 < parts.len() {
                        stdout_redirect = Some((parts[i + 1].clone(), true));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "2>" => {
                    if i + 1 < parts.len() {
                        stderr_redirect = Some((parts[i + 1].clone(), false));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "2>>" => {
                    if i + 1 < parts.len() {
                        stderr_redirect = Some((parts[i + 1].clone(), true));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                _ => {
                    command_args.push(parts[i].clone());
                    i += 1;
                }
            }
        }

        if command_args.is_empty() {
            continue;
        }
        let command = &command_args[0];

        let mut stderr_file = match &stderr_redirect {
            Some((path, append)) => {
                let file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(*append)
                    .truncate(!*append)
                    .open(path);
                match file {
                    Ok(f) => Some(f),
                    Err(e) => {
                        eprintln!("Failed to create stderr file '{}': {}", path, e);
                        None
                    }
                }
            }
            None => None,
        };

        match command.as_str() {
            "exit" => {
                let exit_code = command_args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                process::exit(exit_code);
            }
            "echo" => {
                let output = command_args[1..].join(" ");
                if let Some((file_path, append)) = &stdout_redirect {
                    let mut file = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .append(*append)
                        .truncate(!*append)
                        .open(file_path)
                        .map_err(|e| {
                            let msg = format!("Error opening file: {}", e);
                            if let Some(file) = &mut stderr_file {
                                writeln!(file, "{}", msg).ok();
                            } else {
                                eprintln!("{}", msg);
                            }
                            io::Error::new(io::ErrorKind::Other, e)
                        })?;
                    writeln!(file, "{}", output).map_err(|e| {
                        let msg = format!("Error writing to file: {}", e);
                        if let Some(file) = &mut stderr_file {
                            writeln!(file, "{}", msg).ok();
                        } else {
                            eprintln!("{}", msg);
                        }
                        io::Error::new(io::ErrorKind::Other, e)
                    })?;
                } else {
                    println!("{}", output);
                }
            }
            "type" => {
                if command_args.len() < 2 {
                    continue;
                }
                let cmd_to_check = &command_args[1];
                if builtins.contains(cmd_to_check.as_str()) {
                    if let Some((file_path, append)) = &stdout_redirect {
                        let mut file = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .append(*append)
                            .truncate(!*append)
                            .open(file_path)
                            .map_err(|e| {
                                let msg = format!("Error opening file: {}", e);
                                if let Some(file) = &mut stderr_file {
                                    writeln!(file, "{}", msg).ok();
                                } else {
                                    eprintln!("{}", msg);
                                }
                                io::Error::new(io::ErrorKind::Other, e)
                            })?;
                        writeln!(file, "{} is a shell builtin", cmd_to_check).map_err(|e| {
                            let msg = format!("Error writing to file: {}", e);
                            if let Some(file) = &mut stderr_file {
                                writeln!(file, "{}", msg).ok();
                            } else {
                                eprintln!("{}", msg);
                            }
                            io::Error::new(io::ErrorKind::Other, e)
                        })?;
                    } else {
                        println!("{} is a shell builtin", cmd_to_check);
                    }
                    continue;
                }

                if let Some(path) = find_in_path(cmd_to_check) {
                    if let Some((file_path, append)) = &stdout_redirect {
                        let mut file = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .append(*append)
                            .truncate(!*append)
                            .open(file_path)
                            .map_err(|e| {
                                let msg = format!("Error opening file: {}", e);
                                if let Some(file) = &mut stderr_file {
                                    writeln!(file, "{}", msg).ok();
                                } else {
                                    eprintln!("{}", msg);
                                }
                                io::Error::new(io::ErrorKind::Other, e)
                            })?;
                        writeln!(file, "{} is {}", cmd_to_check, path.display()).map_err(|e| {
                            let msg = format!("Error writing to file: {}", e);
                            if let Some(file) = &mut stderr_file {
                                writeln!(file, "{}", msg).ok();
                            } else {
                                eprintln!("{}", msg);
                            }
                            io::Error::new(io::ErrorKind::Other, e)
                        })?;
                    } else {
                        println!("{} is {}", cmd_to_check, path.display());
                    }
                } else {
                    if let Some((file_path, append)) = &stdout_redirect {
                        let mut file = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .append(*append)
                            .truncate(!*append)
                            .open(file_path)
                            .map_err(|e| {
                                let msg = format!("Error opening file: {}", e);
                                if let Some(file) = &mut stderr_file {
                                    writeln!(file, "{}", msg).ok();
                                } else {
                                    eprintln!("{}", msg);
                                }
                                io::Error::new(io::ErrorKind::Other, e)
                            })?;
                        writeln!(file, "{}: not found", cmd_to_check).map_err(|e| {
                            let msg = format!("Error writing to file: {}", e);
                            if let Some(file) = &mut stderr_file {
                                writeln!(file, "{}", msg).ok();
                            } else {
                                eprintln!("{}", msg);
                            }
                            io::Error::new(io::ErrorKind::Other, e)
                        })?;
                    } else {
                        println!("{}: not found", cmd_to_check);
                    }
                }
            }
            "pwd" => {
                let current_dir = env::current_dir()?;
                if let Some((file_path, append)) = &stdout_redirect {
                    let mut file = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .append(*append)
                        .truncate(!*append)
                        .open(file_path)
                        .map_err(|e| {
                            let msg = format!("Error opening file: {}", e);
                            if let Some(file) = &mut stderr_file {
                                writeln!(file, "{}", msg).ok();
                            } else {
                                eprintln!("{}", msg);
                            }
                            io::Error::new(io::ErrorKind::Other, e)
                        })?;
                    writeln!(file, "{}", current_dir.display()).map_err(|e| {
                        let msg = format!("Error writing to file: {}", e);
                        if let Some(file) = &mut stderr_file {
                            writeln!(file, "{}", msg).ok();
                        } else {
                            eprintln!("{}", msg);
                        }
                        io::Error::new(io::ErrorKind::Other, e)
                    })?;
                } else {
                    println!("{}", current_dir.display());
                }
            }
            "cd" => {
                if command_args.len() != 2 {
                    let msg = format!("cd: expected 1 argument, got {}", command_args.len() - 1);
                    if let Some(file) = &mut stderr_file {
                        writeln!(file, "{}", msg).ok();
                    } else {
                        eprintln!("{}", msg);
                    }
                    continue;
                }
                let new_dir = &command_args[1];
                let path = if new_dir == "~" {
                    match env::var_os("HOME") {
                        Some(home) => PathBuf::from(home),
                        None => {
                            let msg = "cd: HOME environment variable not set";
                            if let Some(file) = &mut stderr_file {
                                writeln!(file, "{}", msg).ok();
                            } else {
                                eprintln!("{}", msg);
                            }
                            continue;
                        }
                    }
                } else {
                    PathBuf::from(new_dir)
                };
                match env::set_current_dir(&path) {
                    Ok(()) => {}
                    Err(e) => {
                        let msg = if e.kind() == io::ErrorKind::NotFound {
                            format!("cd: {}: No such file or directory", path.display())
                        } else {
                            format!("cd: {}", e)
                        };
                        if let Some(file) = &mut stderr_file {
                            writeln!(file, "{}", msg).ok();
                        } else {
                            eprintln!("{}", msg);
                        }
                    }
                }
            }
            _ => {
                if builtins.contains(command.as_str()) {
                    let msg = format!("{}: command not found", command);
                    if let Some(file) = &mut stderr_file {
                        writeln!(file, "{}", msg).ok();
                    } else {
                        eprintln!("{}", msg);
                    }
                    continue;
                }

                let program_path = PathBuf::from(command);
                let program_path_exists = program_path.exists();
                let is_exec = is_executable(&program_path);

                let path = if program_path_exists && is_exec {
                    Some(program_path)
                } else {
                    find_in_path(command)
                };

                if let Some(path) = path {
                    let args = command_args[1..]
                        .iter()
                        .map(|s| OsStr::new(s.as_str()))
                        .collect::<Vec<_>>();

                    let mut cmd = Command::new(&path);
                    cmd.args(args);

                    if let Some((file_path, append)) = &stdout_redirect {
                        match OpenOptions::new()
                            .create(true)
                            .write(true)
                            .append(*append)
                            .truncate(!*append)
                            .open(file_path)
                        {
                            Ok(file) => {
                                cmd.stdout(file);
                            }
                            Err(e) => {
                                let msg = format!("Failed to open stdout file '{}': {}", file_path, e);
                                if let Some(file) = &mut stderr_file {
                                    writeln!(file, "{}", msg).ok();
                                } else {
                                    eprintln!("{}", msg);
                                }
                                continue;
                            }
                        }
                    }

                    if let Some((file_path, append)) = &stderr_redirect {
                        match OpenOptions::new()
                            .create(true)
                            .write(true)
                            .append(*append)
                            .truncate(!*append)
                            .open(file_path)
                        {
                            Ok(file) => {
                                cmd.stderr(file);
                            }
                            Err(e) => {
                                let msg = format!("Failed to open stderr file '{}': {}", file_path, e);
                                eprintln!("{}", msg);
                                continue;
                            }
                        }
                    }

                    #[cfg(unix)]
                    {
                        use std::os::unix::process::CommandExt;
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            cmd.arg0(file_name);
                        }
                    }

                    match cmd.status() {
                        Ok(_status) => {
                            io::stdout().flush()?;
                        }
                        Err(e) => {
                            let msg = format!("{}: command not found", command);
                            if let Some(file) = &mut stderr_file {
                                writeln!(file, "{}", msg).ok();
                            } else {
                                eprintln!("{}", msg);
                            }
                        }
                    }
                } else {
                    let msg = format!("{}: command not found", command);
                    if let Some(file) = &mut stderr_file {
                        writeln!(file, "{}", msg).ok();
                    } else {
                        eprintln!("{}", msg);
                    }
                }
            }
        }
    }

    Ok(())
}