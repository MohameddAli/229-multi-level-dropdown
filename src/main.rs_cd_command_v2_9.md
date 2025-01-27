Okay, I need to add the cd command to the builtins. Let's see how the current code handles builtins. The existing builtins are exit, echo, type, and pwd. So I should add "cd" to the builtins set.

First, I'll add "cd" to the builtins HashSet. The line where they initialize it is here:

let builtins: HashSet<&str> = ["exit", "echo", "type", "pwd"].iter().cloned().collect();

So I'll add "cd" to that array. Then, in the match statement, where the commands are handled, I need to add a case for "cd".

The main thing the cd command does is change the current directory. In Rust, to change the current directory, we can use std::env::set_current_dir. So when the user types "cd /some/path", we need to parse the path argument and call set_current_dir on it.

But wait, the user might not provide any arguments. How does the real cd command behave? If you just type 'cd' with no arguments, it changes to the home directory. Oh right, but according to the problem description, in this stage we're handling absolute paths. So maybe the test cases for this stage don't include the case where no arguments are given. But the problem says to handle absolute paths here. So perhaps in this stage, the tester will only check cases where cd is given an absolute path. But we should still handle the case where the path is given.

So in the code, when the command is "cd", we need to check the arguments. Let's see the code structure. The parts variable is a Vec<&str> split by whitespace. So parts[0] is the command, parts[1] would be the first argument.

So for the "cd" case, we need to get the first argument. If there's no argument, perhaps we should default to the home directory, but according to the problem's notes, maybe we don't handle that yet. But the problem says in this stage we need to handle absolute paths. So perhaps in this stage, the tester will pass cd commands with absolute paths. But what if the user provides more than one argument? Like 'cd a b'? The real cd command would throw an error. So we need to handle that.

So for the 'cd' command:

Check the number of arguments. If there are zero arguments, then according to POSIX, cd with no arguments changes to the home directory. But since the problem says this stage is about absolute paths, maybe the tester doesn't check that. However, the problem statement says that handling the tilde (~) is for later stages. So perhaps in this stage, we can ignore that and require the path to be provided. But the test case in the example has 'cd /usr/local/bin' which is an absolute path, and then another 'cd /does_not_exist' which is also absolute.

So in this stage, the code should:

- When the command is 'cd', check if there is exactly one argument. If not, perhaps output an error. Wait, the problem's test example includes an error message for a non-existent directory, but what about when there are multiple arguments?

In the real shell, if you run 'cd a b', it will say too many arguments. So perhaps in our code, if parts.len() after the command is not 1, we should print an error. For example, in the example given by the problem, when the user inputs 'cd /does_not_exist', which is one argument. If that path does not exist, then the error message is printed.

So first, in the code:

When the command is 'cd', check if parts has exactly one argument (parts.len() == 1? No, because parts includes the command as the first element. So parts.len() should be 2 for exactly one argument. For example, 'cd /usr' would split into parts as ["cd", "/usr"], so parts.len() is 2.

So, in the code:

if parts.len() != 2 {

eprintln!("cd: too many arguments");

continue;

}

But maybe that's not part of the current test. The problem's example shows that when the directory doesn't exist, the error message is "cd: /does_not_exist: No such file or directory". So perhaps in this stage, we only need to handle the case where exactly one argument is provided, which is the path. But if there are zero or multiple arguments, how to handle that?

The problem's example doesn't cover that. But according to the problem's notes, the tester will check the error message when the path is invalid. So for the current stage, perhaps we can assume that the command is called with exactly one argument. But perhaps the problem expects us to handle multiple arguments.

Alternatively, perhaps the code should check that exactly one argument is given, and otherwise, output an error. For example, if parts.len() < 2 (no arguments), then the code would print an error. But according to the problem's test case, the error message is different. So perhaps in our code:

In the 'cd' case:

if parts.len() != 2 {

eprintln!("cd: incorrect number of arguments");

} else {

let path = parts[1];

// check if the path exists and is a directory

let path = Path::new(path);

if path.is_dir() {

if let Err(e) = env::set_current_dir(&path) {

eprintln!("cd: {}: {}", path.display(), e);

}

} else {

eprintln!("cd: {}: No such file or directory", path.display());

}

}

Wait, but how do we check if the path exists? Because even if the path is a directory, but there's a permission issue, set_current_dir would return an error. So perhaps the code should attempt to change the directory and handle any errors from that.

So maybe the code shouldn't check beforehand whether the path exists, but instead just try to set the current directory and handle the error.

So the code for the 'cd' command would be:

match command {

// existing cases...

"cd" => {

if parts.len() != 2 {

eprintln!("cd: expected 1 argument, got {}", parts.len() - 1);

continue;

}

let new_dir = parts[1];

let path = Path::new(new_dir);

if let Err(e) = env::set_current_dir(path) {

// Check the error kind to determine the message

if e.kind() == io::ErrorKind::NotFound {

eprintln!("cd: {}: No such file or directory", new_dir);

} else {

eprintln!("cd: {}", e);

}

}

},

// other commands...

}

But in Rust, the error from set_current_dir could be various. For example, if the path is not a directory, it returns an error. So the code should check the error and print the appropriate message.

But the problem's test case expects the message "No such file or directory" when the path doesn't exist. So perhaps the code can check if the path exists. But how?

Alternatively, the code can attempt to set_current_dir, and if it fails, check the error kind.

In the case of a non-existent path, set_current_dir would return an error with kind NotFound. So the code can match on that and print the appropriate message.

So for example:

use std::io::ErrorKind;

// inside the cd handling...

match env::set_current_dir(path) {

Ok(_) => {},

Err(e) => {

if e.kind() == ErrorKind::NotFound {

eprintln!("cd: {}: No such file or directory", new_dir);

} else {

eprintln!("cd: {}", e);

}

}

}

But perhaps the OS error messages vary. For example, on some systems, trying to cd into a non-directory (like a file) might give a "Not a directory" error. But according to the problem's test example, the error message should be exactly "No such file or directory" when the directory doesn't exist. So in that case, perhaps the code should check if the directory exists and is a directory before attempting to change. But that could lead to race conditions, but for the purpose of this problem, perhaps that's acceptable.

Wait, but checking if the path exists and is a directory doesn't prevent the possibility that between the check and the actual cd, the directory is deleted. But that's a rare case, and for the problem's purposes, it's probably okay.

Alternatively, perhaps it's better to handle the error after attempting to change the directory.

In the example given by the problem, when the user tries to cd into a non-existent directory, the error is "No such file or directory".

So in Rust, when trying to set_current_dir to a path that doesn't exist, the error kind is NotFound. So that's when we can print the appropriate message.

But for other errors, like permission denied, the error message would be different, but the problem's test case doesn't check for those. So perhaps the code can handle the NotFound case and print the required message, and for other errors, just print the error's message.

So the code for the cd command would be:

"cd" => {

if parts.len() != 2 {

eprintln!("cd: expected 1 argument, got {}", parts.len() - 1);

continue;

}

let new_dir = parts[1];

let path = Path::new(new_dir);

match env::set_current_dir(path) {

Ok(()) => (),

Err(e) => {

if e.kind() == io::ErrorKind::NotFound {

eprintln!("cd: {}: No such file or directory", new_dir);

} else {

eprintln!("cd: {}", e);

}

}

}

}

But wait, in the problem's example, the error message is "cd: /does_not_exist: No such file or directory". So the code's error message matches that.

Now, the code needs to check the parts.len() == 2. So if the user enters 'cd' with no arguments, parts.len() is 1, so the code would print "expected 1 argument, got 0".

But according to the problem's description, the current stage is handling absolute paths. So maybe in this stage, the code doesn't handle the case where the user runs 'cd' with no arguments (which in bash would change to the home directory). But the problem's test cases might not test that. However, the problem says that the tester will send a series of cd commands. For example, the first test case is 'cd /usr/local/bin', then 'pwd', which outputs that path.

So in the code, if the user provides no arguments to cd, then the code outputs an error. But in the problem's test example, there's a case where the user provides a single argument. So perhaps that's acceptable.

But wait, in the real 'cd' command, if no arguments are given, it defaults to the home directory. But the problem says that handling the ~ (home directory) is for later stages. So in this stage, the code should not handle that. Therefore, if the user calls 'cd' without arguments, the code will output an error. But according to the problem's test example, maybe the tester isn't checking that case. But according to the problem's note, the tester will send a series of commands. So perhaps the current code for 'cd' in the problem's example is supposed to handle only the case where a single argument (the absolute path) is given.

So the code for the 'cd' command should check if there's exactly one argument. Otherwise, print an error. So in the code:

if parts.len() != 2 {

eprintln!("cd: incorrect number of arguments");

// But according to the problem's example, the error message should be like "No such file or directory".

// Wait, no. If the user provides no arguments, the error message should be different. For example, in bash:

// $ cd

// no error, changes to home directory.

// $ cd a b

// bash: cd: too many arguments

// So in the code, if parts.len() < 2 (no arguments), then the code should perhaps handle that as the home directory. But according to the problem's note, handling ~ (home directory) is for later stages. So perhaps in this stage, the code expects exactly one argument, and otherwise, outputs an error.

// So, for this stage, when the user runs 'cd' with no arguments, the code will print "cd: expected 1 argument, got 0". But according to the problem's test cases, perhaps that's not covered. But according to the problem statement, the tester's example includes a cd command with an argument. So perhaps the code is supposed to handle the case where exactly one argument is given.

// So, the code for 'cd' command is as follows:

// Check if parts.len() is 2 (command + one argument)

// If not, print error message and continue.

}

Therefore, in the code:

Add "cd" to the builtins set.

In the match statement:

"cd" => {

if parts.len() != 2 {

eprintln!("cd: expected 1 argument, got {}", parts.len() - 1);

continue;

}

let new_dir = parts[1];

let path = Path::new(new_dir);

match env::set_current_dir(&path) {

Ok(()) => {},

Err(e) => {

if e.kind() == io::ErrorKind::NotFound {

eprintln!("cd: {}: No such file or directory", new_dir);

} else {

eprintln!("cd: {}", e);

}

}

}

}

Now, testing this code:

If the user runs 'cd /usr/local/bin' (parts.len() is 2), then set_current_dir is called. If it's successful, no output. Then pwd shows the new directory.

If the user runs 'cd /does_not_exist', the set_current_dir returns NotFound, so the error message is printed as per the problem's example.

What about if the user provides a relative path, like 'cd ..'? But according to the problem statement, this stage handles absolute paths. So the code doesn't need to handle relative paths yet (they are for later stages). So the code will attempt to set the current directory to, say, "..", but since that's a relative path, the set_current_dir will resolve it relative to the current directory. But the code would allow that, but according to the problem's description, the current stage is for absolute paths. Hmm. Wait, but the code here just takes the argument as a path. So if the user provides a relative path, like 'cd ..', the code will attempt to change to that directory. But according to the problem's note, this stage covers absolute paths, but the code would also handle relative paths. But perhaps the tester for this stage only tests absolute paths. So the code will handle all paths, but in later stages, the code would need to handle relative paths and ~. For this stage, perhaps the code just needs to implement the basic 'cd' command with error checking as described.

But according to the problem statement, the current stage's tests only check absolute paths. So the code as written should work.

So the steps to add the 'cd' command are:

1. Add "cd" to the builtins set.

2. Add a match case for "cd" in the main loop.

3. Check that exactly one argument is given; otherwise, print an error.

4. Attempt to change the directory using env::set_current_dir.

5. Handle the error, especially NotFound, to print the correct error message.

Now, let's see where to insert this code.

Looking at the existing code, after the "pwd" case, there's a default case for other commands. But since "cd" is in the builtins set, the code will hit the match arm for "cd".

Wait, looking at the code structure:

The main loop reads the input into parts, and the command is parts[0]. Then it matches on command. The existing cases are "exit", "echo", "type", and "pwd". Then there's a default case. So adding a case for "cd" in the match statement.

So:

match command {

"exit" => { ... },

"echo" => { ... },

"type" => { ... },

"pwd" => { ... },

"cd" => { ... },

_ => { ... },

}

So adding a case for "cd" here.

So putting it all together:

In the builtins set:

let builtins: HashSet<&str> = ["exit", "echo", "type", "pwd", "cd"].iter().cloned().collect();

Then in the match:

"cd" => {

if parts.len() != 2 {

eprintln!("cd: expected 1 argument, got {}", parts.len() - 1);

continue;

}

let new_dir = parts[1];

let path = Path::new(new_dir);

match env::set_current_dir(&path) {

Ok(()) => {},

Err(e) => {

if e.kind() == io::ErrorKind::NotFound {

eprintln!("cd: {}: No such file or directory", new_dir);

} else {

eprintln!("cd: {}", e);

}

}

}

}

This should handle the cd command with absolute paths.

Testing this code:

When the user inputs 'cd /valid/path', the current directory changes.

If the path is invalid, the error is printed as per the problem's example.

If the user inputs 'cd' with no arguments, the code prints an error about incorrect number of arguments.

But according to the problem's test example, the tester checks the error message when the path is invalid. So the code should handle that case correctly.

Now, integrate this into the existing code.

Also, check that other commands like 'pwd' are not broken. The 'pwd' command uses env::current_dir(), which should reflect the changed directory after a 'cd' command.

Thus, the code should work.


To solve this problem, we need to implement the `cd` builtin command in a simple shell program. The `cd` command allows changing the current working directory using absolute paths. The solution should handle valid paths correctly and print appropriate error messages for invalid paths.

### Approach
1. **Add `cd` to Builtins**: Include `cd` in the set of built-in commands recognized by the shell.
2. **Argument Handling**: Ensure the `cd` command receives exactly one argument (the path). If not, print an error message.
3. **Change Directory**: Use Rust's `std::env::set_current_dir` to attempt changing the directory. Handle errors, particularly for non-existent paths, by checking the error kind and printing the appropriate message.

### Solution Code
```rust
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
            "pwd" => {
                let current_dir = env::current_dir()?;
                println!("{}", current_dir.display());
            }
            "cd" => {
                if parts.len() != 2 {
                    eprintln!("cd: expected 1 argument, got {}", parts.len() - 1);
                    continue;
                }
                let new_dir = parts[1];
                let path = Path::new(new_dir);
                match env::set_current_dir(&path) {
                    Ok(()) => {}
                    Err(e) => {
                        if e.kind() == io::ErrorKind::NotFound {
                            eprintln!("cd: {}: No such file or directory", new_dir);
                        } else {
                            eprintln!("cd: {}", e);
                        }
                    }
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

                let args = parts
                    .iter()
                    .skip(1)
                    .map(|s| OsStr::new(s))
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
```

### Explanation
1. **Built-in Command Recognition**: The `cd` command is added to the set of built-in commands, allowing the shell to recognize it.
2. **Argument Validation**: The `cd` command checks if exactly one argument is provided. If not, an error message is printed.
3. **Directory Change**: The shell attempts to change the directory using `std::env::set_current_dir`. If the directory does not exist, a specific error message is printed. Other errors are handled generically.
4. **Integration**: The solution integrates seamlessly with existing commands like `pwd`, ensuring the current directory is accurately reflected after changes.

This approach ensures the shell correctly handles absolute paths with `cd`, providing appropriate feedback for valid and invalid operations.


