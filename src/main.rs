use core::str;
use std::{
    cell::RefCell,
    collections::HashSet,
    env,
    fs,
    io::{self, Write},
    os::unix::process::CommandExt, // Added import for CommandExt
    path::PathBuf,
};
use rustyline::{
    completion::{Completer, Pair},
    error::ReadlineError,
    Context, Editor, Helper,
};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use std::process::Output;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::process::Stdio;
use std::string::FromUtf8Error;
use std::env::VarError;

const BUILTIN_COMMANDS: [&str; 2] = ["echo", "exit"];

#[derive(Default)]
struct ShellCompleter {
    state: RefCell<CompletionState>,
}

#[derive(Default)] // Added Default derive to CompletionState
struct CompletionState {
    last_line: String,
    last_pos: usize,
    tab_count: usize,
    matches: Vec<Pair>,
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let start = line[..pos].rfind(char::is_whitespace).map_or(0, |i| i + 1);
        let word = &line[start..pos];

        // Collect built-in commands
        let built_in_matches: Vec<Pair> = BUILTIN_COMMANDS
            .iter()
            .filter(|&cmd| cmd.starts_with(word))
            .map(|&cmd| Pair {
                display: cmd.to_string(),
                replacement: format!("{} ", cmd),
            })
            .collect();

        // Collect external executables
        let mut external_commands = Vec::new();
        if let Ok(path_var) = env::var("PATH") {
            for dir in path_var.split(':') {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() || file_type.is_symlink() {
                                if let Some(name) = entry.file_name().to_str() {
                                    if !BUILTIN_COMMANDS.contains(&name) {
                                        external_commands.push(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Deduplicate and filter external commands
        let mut seen = HashSet::new();
        let mut unique_externals: Vec<String> = external_commands
            .into_iter()
            .filter(|cmd| {
                if seen.contains(cmd) {
                    false
                } else {
                    seen.insert(cmd.clone());
                    true
                }
            })
            .filter(|cmd| cmd.starts_with(word))
            .collect();
        unique_externals.sort();

        let external_matches: Vec<Pair> = unique_externals
            .iter()
            .map(|cmd| Pair {
                display: cmd.to_string(),
                replacement: format!("{} ", cmd),
            })
            .collect();

        let mut all_matches = [built_in_matches, external_matches].concat();
        all_matches.sort_by(|a, b| a.display.cmp(&b.display));

        // Update completion state
        let mut state = self.state.borrow_mut();
        if state.last_line == line && state.last_pos == pos {
            state.tab_count += 1;
        } else {
            state.last_line = line.to_string();
            state.last_pos = pos;
            state.tab_count = 1;
            state.matches = all_matches.clone();
        }

        // Compute longest common prefix (without trailing space)
        let candidate_strings: Vec<String> = all_matches
            .iter()
            .map(|p| p.replacement.trim_end().to_string())
            .collect();
        let mut lcp = longest_common_prefix(&candidate_strings);

        // If we can extend the current word, complete to the longest common prefix.
        if lcp.len() > word.len() {
            // If there's a unique match, add trailing space.
            if all_matches.len() == 1 {
                lcp.push(' ');
            }
            return Ok((start, vec![Pair {
                display: lcp.clone(),
                replacement: lcp,
            }]));
        }

        // If multiple matches exist, handle repeated Tab presses.
        if all_matches.len() > 1 {
            if state.tab_count == 1 {
                // First TAB: ring bell
                print!("\x07");
                io::stdout().flush().ok();
                Ok((pos, vec![]))
            } else {
                // Second TAB: print suggestions (separated by 2 spaces) and reprint prompt.
                println!();
                for pair in &state.matches {
                    print!("{}  ", pair.display);
                }
                println!();
                io::stdout().flush().ok();
                print!("$ {}", line);
                io::stdout().flush().ok();
                Ok((pos, vec![]))
            }
        } else {
            // Only one match available.
            Ok((start, all_matches))
        }
    }
}

// Helper function: compute longest common prefix of given strings.
fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let mut prefix = strings[0].clone();
    for s in strings.iter().skip(1) {
        while !s.starts_with(&prefix) {
            if prefix.is_empty() {
                break;
            }
            prefix.pop();
        }
    }
    prefix
}

impl Helper for ShellCompleter {}

impl Highlighter for ShellCompleter {}
impl Hinter for ShellCompleter {
    type Hint = String;
}
impl Validator for ShellCompleter {}

#[derive(Debug, PartialEq, Eq)]

enum ShellExec {

    PrintToStd(Command),

    RedirectedStdOut(Command, PathBuf),

    RedirectedStdErr(Command, PathBuf),

    RedirectedStdOutAppend(Command, PathBuf),

    RedirectedStdErrAppend(Command, PathBuf),

}

#[derive(Debug, PartialEq, Eq)]

enum Command {

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

fn main() -> Result<()> {

    let home = env::var("HOME")?;

    let path = env::var("PATH")?;

    let mut rl = Editor::new()?;

    rl.set_helper(Some(ShellCompleter::default()));

    loop {

        match rl.readline("$ ") {

            Ok(line) => {

                let trimmed_input = line.trim();

                if !trimmed_input.is_empty() {  // Removed redundant parentheses

                    rl.add_history_entry(trimmed_input);

                }

                let exec = parse(trimmed_input);

                match exec {

                    ShellExec::PrintToStd(Command::Exit(s)) if s == "0" => break,

                    ShellExec::RedirectedStdOut(Command::Exit(s), _) if s == "0" => break,

                    ShellExec::PrintToStd(Command::Empty) => continue,

                    ShellExec::RedirectedStdOut(Command::Empty, _) => continue,

                    ShellExec::PrintToStd(c) => {

                        let output = exec_command(c, &path, &home)?;

                        match output {

                            CommandOutput::StdOut(s) => println!("{}", s),

                            CommandOutput::StdErr(s) => eprintln!("{}", s),

                            CommandOutput::Wrapped(c, output) => {

                                if !output.stdout.is_empty() {

                                    println!("{}", String::from_utf8(output.stdout)?.trim())

                                }

                                if !output.stderr.is_empty() {

                                    print_sys_program_failure_to_stderr(c, output.stderr)?

                                }

                            }

                            CommandOutput::Noop => continue,

                        }

                    }

                    ShellExec::RedirectedStdOut(command, file) => {

                        let file = File::create(file)?;

                        let output = exec_command(command, &path, &home)?;

                        handle_redirected_std_out(file, output)?

                    }

                    ShellExec::RedirectedStdOutAppend(command, file) => {

                        let file = OpenOptions::new().append(true).create(true).open(file)?;

                        let output = exec_command(command, &path, &home)?;

                        handle_redirected_std_out(file, output)?

                    }

                    ShellExec::RedirectedStdErr(command, file) => {

                        let file = File::create(file)?;

                        let output = exec_command(command, &path, &home)?;

                        handle_redirected_std_err(file, output)?

                    }

                    ShellExec::RedirectedStdErrAppend(command, file) => {

                        let file = OpenOptions::new().append(true).create(true).open(file)?;

                        let output = exec_command(command, &path, &home)?;

                        handle_redirected_std_err(file, output)?

                    }

                }

            }

            Err(ReadlineError::Interrupted) => continue,

            Err(ReadlineError::Eof) => break,

            Err(err) => {

                eprintln!("Error: {}", err);

                break;

            }

        }

    }

    Ok(())

}

fn handle_redirected_std_out(mut file: File, output: CommandOutput) -> Result<()> {

    match output {

        CommandOutput::StdOut(s) => {

            writeln!(file, "{}", s)?;

            file.flush()?

        }

        CommandOutput::StdErr(s) => eprintln!("{}", s),

        CommandOutput::Wrapped(c, output) => {

            if !output.stdout.is_empty() { // Removed redundant parentheses

                writeln!(file, "{}", String::from_utf8(output.stdout)?.trim())?;

                file.flush()?

            }

            if !output.stderr.is_empty() { // Removed redundant parentheses

                print_sys_program_failure_to_stderr(c, output.stderr)?

            }

        }

        CommandOutput::Noop => (),

    }

    Ok(())

}

fn handle_redirected_std_err(mut file: File, output: CommandOutput) -> Result<()> {

    match output {

        CommandOutput::StdOut(s) => eprintln!("{}", s),

        CommandOutput::StdErr(s) => writeln!(file, "{}", s)?,

        CommandOutput::Wrapped(c, output) => {

            if !output.stdout.is_empty() {

                println!("{}", String::from_utf8(output.stdout)?.trim())

            }

            if !output.stderr.is_empty() {

                let raw_error_message = String::from_utf8(output.stderr)?;

                if let Some(split_point) = raw_error_message.find(&c) {

                    if let Some((_, right_half)) = raw_error_message.split_at_checked(split_point) {

                        let err_msg = &right_half[c.len()..];

                        writeln!(file, "{}{}", c, err_msg.trim())?;

                    } else {

                        writeln!(file, "{}", raw_error_message.trim())?;

                    }

                } else {

                    writeln!(file, "{}", raw_error_message.trim())?;

                }

                file.flush()?;

            }

        }

        CommandOutput::Noop => (),

    }

    Ok(())

}

fn print_sys_program_failure_to_stderr(program: String, stderr: Vec<u8>) -> Result<()> {

    let raw_error_message = String::from_utf8(stderr)?;

    if let Some(split_point) = raw_error_message.find(&program) {

        if let Some((_, right_half)) = raw_error_message.split_at_checked(split_point) {

            let err_msg = &right_half[program.len()..];

            eprintln!("{}{}", program, err_msg.trim());

        } else {

            eprintln!("{}", raw_error_message.trim());

        }

    } else {

        eprintln!("{}", raw_error_message.trim());

    }

    Ok(())

}

fn exec_command(command: Command, path: &str, home: &str) -> Result<CommandOutput> {

    let built_in_commands = ["echo", "exit", "type", "pwd", "cd"];

    match command {

        Command::Exit(s) if s == "0" => Ok(CommandOutput::Noop),

        Command::Exit(s) => Ok(CommandOutput::StdErr(format!("Unknown exit code {}", s))),

        Command::Echo(s) => Ok(CommandOutput::StdOut(s.to_string())),

        Command::Type(c) if built_in_commands.contains(&c.as_str()) => {

            Ok(CommandOutput::StdOut(format!("{} is a shell builtin", c)))

        }

        Command::Type(c) => {

            if !c.is_empty() { // Removed redundant parentheses

                if let Some(executable) = find_executable_on_path(path, &c)? {

                    Ok(CommandOutput::StdOut(format!(

                        "{} is {}",

                        c,

                        executable.display()

                    )))

                } else {

                    Ok(CommandOutput::StdErr(format!("{}: not found", c)))

                }

            } else {

                Ok(CommandOutput::Noop)

            }

        }

        Command::Pwd => {

            let curren_dir = env::current_dir()?;

            Ok(CommandOutput::StdOut(format!("{}", curren_dir.display())))

        }

        Command::Cd(directory) => {

            if !directory.is_empty() {

                let dir_path = if directory == "~" { // Removed redundant parentheses

                    Path::new(home)

                } else {

                    Path::new(&directory)

                };

                if env::set_current_dir(dir_path).is_err() {

                    Ok(CommandOutput::StdOut(format!(

                        "cd: {}: No such file or directory",

                        directory

                    )))

                } else {

                    Ok(CommandOutput::Noop)

                }

            } else {

                Ok(CommandOutput::Noop)

            }

        }

        Command::SysProgram(c, args) => {

            if let Some(program) = find_executable_on_path(path, &c)? {

                let output = run_executable_with_args(&program, args.as_slice())?;

                Ok(CommandOutput::Wrapped(c.to_string(), output))

            } else {

                Ok(CommandOutput::StdErr(format!("{}: command not found", c)))

            }

        }

        Command::Empty => Ok(CommandOutput::Noop),

        Command::Invalid => Err(Error::InvalidCommand),

    }

}

fn parse(input: &str) -> ShellExec {

    let tokens = tokenize(input);

    if let Some((split_point, _)) = tokens

        .iter()

        .enumerate()

        .find(|&(_, token)| token == "1>" || token == ">")

    {

        if let Some((left_half, right_half)) = tokens.split_at_checked(split_point) {

            let command = &left_half[..left_half.len()];

            let file = &right_half[1..];

            let command = redirected_command(command.to_vec());

            ShellExec::RedirectedStdOut(command, PathBuf::from(file.join(" ")))

        } else {

            ShellExec::PrintToStd(Command::Invalid)

        }

    } else if let Some((split_point, _)) = tokens

        .iter()

        .enumerate()

        .find(|&(_, token)| token == "1>>" || token == ">>")

    {

        if let Some((left_half, right_half)) = tokens.split_at_checked(split_point) {

            let command = &left_half[..left_half.len()];

            let file = &right_half[1..];

            let command = redirected_command(command.to_vec());

            ShellExec::RedirectedStdOutAppend(command, PathBuf::from(file.join(" ")))

        } else {

            ShellExec::PrintToStd(Command::Invalid)

        }

    } else if let Some((split_point, _)) =

        tokens.iter().enumerate().find(|&(_, token)| token == "2>")

    {

        if let Some((left_half, right_half)) = tokens.split_at_checked(split_point) {

            let command = &left_half[..left_half.len()];

            let file = &right_half[1..];

            let command = redirected_command(command.to_vec());

            ShellExec::RedirectedStdErr(command, PathBuf::from(file.join(" ")))

        } else {

            ShellExec::PrintToStd(Command::Invalid)

        }

    } else if let Some((split_point, _)) =

        tokens.iter().enumerate().find(|&(_, token)| token == "2>>")

    {

        if let Some((left_half, right_half)) = tokens.split_at_checked(split_point) {

            let command = &left_half[..left_half.len()];

            let file = &right_half[1..];

            let command = redirected_command(command.to_vec());

            ShellExec::RedirectedStdErrAppend(command, PathBuf::from(file.join(" ")))

        } else {

            ShellExec::PrintToStd(Command::Invalid)

        }

    } else if let Some((head, tail)) = tokens.split_first() {

        let command = match head.as_str() {

            "echo" => Command::Echo(tail.join(" ")),

            "exit" => Command::Exit(tail.join(" ")),

            "type" => Command::Type(tail.join(" ")),

            "pwd" => Command::Pwd,

            "cd" => Command::Cd(tail.join(" ")),

            c => Command::SysProgram(c.to_owned(), tail.to_vec()),

        };

        ShellExec::PrintToStd(command)

    } else {

        ShellExec::PrintToStd(Command::Empty)

    }

}

fn redirected_command(command: Vec<String>) -> Command {

    if let Some((head, tail)) = command.split_first() {

        match head.as_str() {

            "echo" => Command::Echo(tail.join(" ")),

            "exit" => Command::Exit(tail.join(" ")), 

            "type" => Command::Type(tail.join(" ")),

            "pwd" => Command::Pwd,

            "cd" => Command::Cd(tail.join(" ")),

            c => Command::SysProgram(c.to_owned(), tail.to_vec()),

        }

    } else {

        Command::Invalid

    }

}

fn tokenize(input: &str) -> Vec<String> {

    let mut tokens = Vec::new();

    let mut current_token = String::new();

    let mut in_single_quote = false;

    let mut in_double_quote = false;

    let mut in_escape = false;

    for (i, c) in input.chars().enumerate() {

        match c {

            '\\' if in_escape => {

                current_token.push(c);

                in_escape = false;

            }

            '\\' if !in_single_quote && !in_double_quote => in_escape = true,

            '\\' if in_double_quote => {

                if let Some(next_char) = input.chars().nth(i + 1) {

                    if next_char == '$'

                        || next_char == '\\'

                        || next_char == '"'

                        || next_char == '\n'

                    {

                        in_escape = true;

                    } else {

                        current_token.push(c);

                    }

                }

            }

            '\'' if in_escape => {

                current_token.push(c);

                in_escape = false;

            }

            '\'' if in_single_quote => {

                in_single_quote = false;

            }

            '\'' if !in_double_quote => in_single_quote = true,

            '"' if in_escape => {

                current_token.push(c);

                in_escape = false;

            }

            '"' if in_double_quote => {

                in_double_quote = false;

            }

            '"' if !in_single_quote => in_double_quote = true,

            ' ' | '\t' if in_escape => {

                current_token.push(c);

                in_escape = false;

            }

            ' ' | '\t' if !in_single_quote && !in_double_quote => {

                if !current_token.is_empty() {

                    tokens.push(current_token.clone());

                    current_token.clear();

                }

            }

            ' ' | '\t' => current_token.push(c),

            '\n' if in_escape => in_escape = false,

            _ => {

                current_token.push(c);

                in_escape = false;

            }

        }

    }

    // Don't forget to add the last word if there is one

    if !current_token.is_empty() {

        tokens.push(current_token);

    }

    tokens

}

fn find_executable_on_path(path: &str, executable: &str) -> Result<Option<PathBuf>> {

    Ok(path

        .split(":")

        .map(|dir| Path::new(dir).join(executable))

        .find(|path| fs::metadata(path).is_ok()))

}

fn run_executable_with_args(program: &PathBuf, args: &[String]) -> io::Result<Output> {

    std::process::Command::new(program)

        .arg0(program.file_name().unwrap().to_str().unwrap())

        .args(args)

        .stdout(Stdio::piped())

        .output()

}

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]

#[allow(dead_code)]

pub enum Error {

    InvalidCommand,

    EncodingError(FromUtf8Error),

    EnvVarError(VarError),

    Io(std::io::Error),

    ReadlineError(ReadlineError),

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

impl From<ReadlineError> for Error {

    fn from(value: ReadlineError) -> Self {

        Self::ReadlineError(value)

    }

}

impl core::fmt::Display for Error {

    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        write!(fmt, "{self:?}")

    }

}

impl std::error::Error for Error {}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]

    fn tokenize_should_split_on_whitespace() {

        let result = tokenize("echo foo     bar asd");

        assert_eq!(result, vec!["echo", "foo", "bar", "asd"]);

    }

    #[test]

    fn tokenize_should_preserve_all_characters_in_single_quotes() {

        let test_cases = vec![

            (

                "echo 'foo                  bar'",

                vec!["echo", "foo                  bar"],

            ),

            (

                "cat '/tmp/file name' '/tmp/file name with spaces'",

                vec!["cat", "/tmp/file name", "/tmp/file name with spaces"],

            ),

            (r#"echo 'foo\     bar'"#, vec!["echo", r#"foo\     bar"#]),

            (r#"echo 'foo" bar'"#, vec!["echo", r#"foo" bar"#]),

            (r#"'exe with "quotes"'"#, vec![r#"exe with "quotes""#]),

        ];

        for (test_case, expected_result) in test_cases {

            assert_eq!(tokenize(test_case), expected_result);

        }

    }

    #[test]

    fn tokenize_should_preserve_most_characters_in_double_quotes_except_backslash_in_cases_followed_by_speciacial_character(

    ) {

        let test_cases = vec![

            (r#"echo "/""#, vec!["echo", "/"]),

            (r#"echo "foo"bar"#, vec!["echo", "foobar"]),

            (

                r#"echo "quz  hello"  "bar""#,

                vec!["echo", "quz  hello", "bar"],

            ),

            (

                r#"echo "bar"  "shell's"  "foo""#,

                vec!["echo", "bar", "shell's", "foo"],

            ),

            (

                r#"cat "/tmp/file name" "/tmp/'file name' with spaces""#,

                vec!["cat", "/tmp/file name", "/tmp/'file name' with spaces"],

            ),

            (

                r#"echo "before\   after""#,

                vec!["echo", r#"before\   after"#],

            ),

            (

                r#"cat "/tmp/file\\name" "/tmp/file\ name""#,

                vec!["cat", r#"/tmp/file\name"#, r#"/tmp/file\ name"#],

            ),

            (

                r#"echo "hello'script'\\n'world""#,

                vec!["echo", r#"hello'script'\n'world"#],

            ),

            (r#"echo "\""#, vec!["echo", r#"""#]),

            (

                r#"echo "hello\"insidequotes"script\""#,

                vec!["echo", r#"hello"insidequotesscript""#],

            ),

            (

                r#"cat "/tmp/"file\name"" "/tmp/"file name"""#,

                vec!["cat", "/tmp/filename", "/tmp/file", "name"],

            ),

            (

                r#""exe with 'single quotes'""#,

                vec!["exe with 'single quotes'"],

            ),

            (

                r#"echo "mixed\"quote'example'\\""#,

                vec!["echo", r#"mixed"quote'example'\"#],

            ),

        ];

        for (test_case, expected_result) in test_cases {

            assert_eq!(tokenize(test_case), expected_result);

        }

    }

    #[test]

    fn tokenize_should_treat_unquoted_backslash_as_escape_character() {

        let test_cases = vec![

            (r#"echo \"#, vec!["echo"]),

            (r#"echo \'"#, vec!["echo", r#"'"#]),

            (r#"echo script\""#, vec!["echo", r#"script""#]),

            (

                r#"echo world\ \ \ \ \ \ script"#,

                vec!["echo", "world      script"],

            ),

            (r#"cat file\ name"#, vec!["cat", "file name"]),

            (

                r#"echo \'\"test example\"\'"#,

                vec!["echo", r#"'"test"#, r#"example"'"#],

            ),

        ];

        for (test_case, expected_result) in test_cases {

            assert_eq!(tokenize(test_case), expected_result);

        }

    }

    #[test]

    fn parse_into_command_should_return_redirect_std_out_in_case_tokens_contain_redirection_operator(

    ) {

        let test_cases = vec![

            (

                "ls /tmp/baz > /tmp/foo/baz.md",

                ShellExec::RedirectedStdOut(

                    Command::SysProgram(String::from("ls"), vec![String::from("/tmp/baz")]),

                    PathBuf::from("/tmp/foo/baz.md"),

                ),

            ),

            (

                "ls /tmp/baz 1> /tmp/foo/baz.md",

                ShellExec::RedirectedStdOut(

                    Command::SysProgram(String::from("ls"), vec![String::from("/tmp/baz")]),

                    PathBuf::from("/tmp/foo/baz.md"),

                ),

            ),

        ];

        for (test_case, expected_result) in test_cases {

            assert_eq!(parse(test_case), expected_result);

        }

    }

    #[test]

    fn parse_into_command_should_return_redirect_std_err_in_case_tokens_contain_redirection_operator(

    ) {

        let test_cases = vec![(

            "ls /tmp/baz 2> /tmp/foo/baz.md",

            ShellExec::RedirectedStdErr(

                Command::SysProgram(String::from("ls"), vec![String::from("/tmp/baz")]),

                PathBuf::from("/tmp/foo/baz.md"),

            ),

        )];

        for (test_case, expected_result) in test_cases {

            assert_eq!(parse(test_case), expected_result);

        }

    }

    #[test]

    fn parse_into_command_should_return_redirect_std_out_in_case_tokens_contain_appending_redirection_operator(

    ) {

        let test_cases = vec![

            (

                "ls /tmp/baz >> /tmp/foo/baz.md",

                ShellExec::RedirectedStdOutAppend(

                    Command::SysProgram(String::from("ls"), vec![String::from("/tmp/baz")]),

                    PathBuf::from("/tmp/foo/baz.md"),

                ),

            ),

            (

                "ls /tmp/baz 1>> /tmp/foo/baz.md",

                ShellExec::RedirectedStdOutAppend(

                    Command::SysProgram(String::from("ls"), vec![String::from("/tmp/baz")]),

                    PathBuf::from("/tmp/foo/baz.md"),

                ),

            ),

        ];

        for (test_case, expected_result) in test_cases {

            assert_eq!(parse(test_case), expected_result);

        }

    }

    #[test]

    fn parse_into_command_should_return_redirect_std_err_in_case_tokens_contain_appending_redirection_operator(

    ) {

        let test_cases = vec![(

            "ls /tmp/baz 2>> /tmp/foo/baz.md",

            ShellExec::RedirectedStdErrAppend(

                Command::SysProgram(String::from("ls"), vec![String::from("/tmp/baz")]),

                PathBuf::from("/tmp/foo/baz.md"),

            ),

        )];

        for (test_case, expected_result) in test_cases {

            assert_eq!(parse(test_case), expected_result);

        }

    }

}