use std::collections::HashSet;
use std::env::{self, VarError};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output};
use std::string::FromUtf8Error;

#[derive(Debug, PartialEq, Eq)]
enum ShellExec {
    PrintToStd(ShellCommand),
    RedirectedStdOut(ShellCommand, PathBuf),
    RedirectedStdErr(ShellCommand, PathBuf),
    RedirectedStdOutAppend(ShellCommand, PathBuf),
    RedirectedStdErrAppend(ShellCommand, PathBuf),
}

#[derive(Debug, PartialEq, Eq)]
enum ShellCommand {
    Exit(String),
    Echo(String),
    Type(String),
    Pwd,
    Cd(String),
    SysProgram(String, Vec<String>),
    Empty,
    Invalid,
}

#[derive(Debug, PartialEq, Eq)]
enum CommandOutput {
    StdOut(String),
    StdErr(String),
    Wrapped(String, Output),
    Noop,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidCommand,
    EncodingError(FromUtf8Error),
    EnvVarError(VarError),
    Io(std::io::Error),
}

impl From<VarError> for Error {
    fn from(value: VarError) -> Self {
        Self::EnvVarError(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(value: FromUtf8Error) -> Self {
        Self::EncodingError(value)
    }
}

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

fn handle_command_output(output: CommandOutput, stdout_file: Option<&mut File>, stderr_file: Option<&mut File>) -> Result<()> {
    match output {
        CommandOutput::StdOut(s) => {
            if let Some(file) = stdout_file {
                writeln!(file, "{}", s)?;
            } else {
                println!("{}", s);
            }
        }
        CommandOutput::StdErr(s) => {
            if let Some(file) = stderr_file {
                writeln!(file, "{}", s)?;
            } else {
                eprintln!("{}", s);
            }
        }
        CommandOutput::Wrapped(cmd, output) => {
            if !output.stdout.is_empty() {
                let stdout_str = String::from_utf8(output.stdout)?.trim().to_string();
                if let Some(file) = stdout_file {
                    writeln!(file, "{}", stdout_str)?;
                } else {
                    println!("{}", stdout_str);
                }
            }
            if !output.stderr.is_empty() {
                let stderr_str = String::from_utf8(output.stderr)?.trim().to_string();
                if let Some(file) = stderr_file {
                    writeln!(file, "{}{}", cmd, stderr_str)?;
                } else {
                    eprintln!("{}{}", cmd, stderr_str);
                }
            }
        }
        CommandOutput::Noop => {}
    }
    Ok(())
}

fn execute_shell_command(command: ShellCommand, path: &str, home: &str) -> Result<CommandOutput> {
    match command {
        ShellCommand::Exit(code) => {
            let exit_code = code.parse().unwrap_or(0);
            process::exit(exit_code);
        }
        ShellCommand::Echo(s) => Ok(CommandOutput::StdOut(s)),
        ShellCommand::Type(cmd) => {
            let builtins: HashSet<&str> = ["exit", "echo", "type", "pwd", "cd"].iter().cloned().collect();
            if builtins.contains(cmd.as_str()) {
                Ok(CommandOutput::StdOut(format!("{} is a shell builtin", cmd)))
            } else if let Some(path) = find_in_path(&cmd) {
                Ok(CommandOutput::StdOut(format!("{} is {}", cmd, path.display())))
            } else {
                Ok(CommandOutput::StdOut(format!("{}: not found", cmd)))
            }
        }
        ShellCommand::Pwd => {
            let current_dir = env::current_dir()?;
            Ok(CommandOutput::StdOut(format!("{}", current_dir.display())))
        }
        ShellCommand::Cd(dir) => {
            let path = if dir == "~" {
                PathBuf::from(home)
            } else {
                PathBuf::from(dir)
            };
            
            match env::set_current_dir(&path) {
                Ok(()) => Ok(CommandOutput::Noop),
                Err(e) => Ok(CommandOutput::StdErr(format!("cd: {}: {}", path.display(), e)))
            }
        }
        ShellCommand::SysProgram(cmd, args) => {
            if let Some(program_path) = find_in_path(&cmd) {
                let output = Command::new(&program_path)
                    .args(&args)
                    .output()?;
                Ok(CommandOutput::Wrapped(cmd, output))
            } else {
                Ok(CommandOutput::StdErr(format!("{}: command not found", cmd)))
            }
        }
        ShellCommand::Empty => Ok(CommandOutput::Noop),
        ShellCommand::Invalid => Err(Error::InvalidCommand),
    }
}

fn main() -> Result<()> {
    let home = env::var("HOME")?;
    let path = env::var("PATH")?;

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

        let shell_exec = parse_shell_exec(&parts);
        
        match shell_exec {
            ShellExec::PrintToStd(cmd) => {
                let output = execute_shell_command(cmd, &path, &home)?;
                handle_command_output(output, None, None)?;
            }
            ShellExec::RedirectedStdOut(cmd, file_path) => {
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&file_path)?;
                let output = execute_shell_command(cmd, &path, &home)?;
                handle_command_output(output, Some(&mut file), None)?;
            }
            ShellExec::RedirectedStdOutAppend(cmd, file_path) => {
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(&file_path)?;
                let output = execute_shell_command(cmd, &path, &home)?;
                handle_command_output(output, Some(&mut file), None)?;
            }
            ShellExec::RedirectedStdErr(cmd, file_path) => {
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&file_path)?;
                let output = execute_shell_command(cmd, &path, &home)?;
                handle_command_output(output, None, Some(&mut file))?;
            }
            ShellExec::RedirectedStdErrAppend(cmd, file_path) => {
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(&file_path)?;
                let output = execute_shell_command(cmd, &path, &home)?;
                handle_command_output(output, None, Some(&mut file))?;
            }
        }
    }

    Ok(())
}

fn parse_shell_exec(parts: &[String]) -> ShellExec {
    let mut command_parts = Vec::new();
    let mut stdout_redirect = None;
    let mut stderr_redirect = None;
    let mut is_append = false;

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
                command_parts.push(parts[i].clone());
                i += 1;
            }
        }
    }

    if command_parts.is_empty() {
        return ShellExec::PrintToStd(ShellCommand::Empty);
    }

    let command = match command_parts[0].as_str() {
        "exit" => ShellCommand::Exit(command_parts[1..].join(" ")),
        "echo" => ShellCommand::Echo(command_parts[1..].join(" ")),
        "type" => ShellCommand::Type(command_parts[1..].join(" ")),
        "pwd" => ShellCommand::Pwd,
        "cd" => ShellCommand::Cd(command_parts[1..].join(" ")),
        cmd => ShellCommand::SysProgram(cmd.to_string(), command_parts[1..].to_vec()),
    };

    match (stdout_redirect, stderr_redirect) {
        (Some((path, true)), _) => ShellExec::RedirectedStdOutAppend(command, PathBuf::from(path)),
        (Some((path, false)), _) => ShellExec::RedirectedStdOut(command, PathBuf::from(path)),
        (None, Some((path, true))) => ShellExec::RedirectedStdErrAppend(command, PathBuf::from(path)),
        (None, Some((path, false))) => ShellExec::RedirectedStdErr(command, PathBuf::from(path)),
        (None, None) => ShellExec::PrintToStd(command),
    }
}
