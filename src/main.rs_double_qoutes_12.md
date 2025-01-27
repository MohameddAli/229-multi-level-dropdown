Okay, I need to add support for double quotes in the shell. The current code handles single quotes, but now I need to handle double quotes as well. Let me think about how to approach this.

First, the problem says that inside double quotes, characters are preserved except for backslash, which has special meaning when followed by certain characters like \, $, ", or newline. Wait, but according to the task description, in double quotes, the backslash retains its special meaning only when followed by those characters. So inside double quotes, backslashes before other characters are treated as literal, right?

Wait, the user's note says: "Enclosing characters in double quotes preserves the literal value of each character within the quotes except \, the backslash retains its special meaning when followed by \, $, " or newline." So, in double quotes, a backslash is only an escape character if it's before one of those characters. Otherwise, it's just a regular backslash. Hmm. So for example, if I have "\$", it would escape the $, so $ is treated as a literal. But if I have "\a", the backslash is just a backslash and an 'a'.

So, the parser needs to handle double quotes by capturing everything inside them, processing backslash escapes for those specific characters. Also, variables (like $VAR) are not mentioned here, but since the shell isn't handling variables yet, maybe we can ignore that part for now. The main thing is to handle the quotes and the backslash escaping.

The current parse_arguments function handles single quotes by toggling a flag when a quote is encountered. Now, I need to modify this function to also handle double quotes and the escaping rules for them.

Let me look at the existing parse_arguments function. It uses a state variable in_quotes (a boolean) which is set when a single quote is encountered. The code for single quotes is straightforward: when in_quotes is true, it just captures characters until the next single quote. But for double quotes, it's more complex because of the escape characters.

So, the plan is to modify the parse_arguments function to track whether we are inside single quotes, double quotes, or not in quotes. Then, when inside double quotes, process backslash escapes as per the rules.

Wait, but how to handle nested quotes or different quote types. For example, double quotes inside single quotes are treated as literals, and vice versa. So the state should track which type of quote we are in. So instead of a boolean in_quotes, maybe have an enum like QuoteState { None, Single, Double }.

So, let's adjust the code:

- Change in_quotes from a boolean to a QuoteState (None, Single, Double).

- When a quote is encountered, if not in any quotes, switch to the corresponding quote state. If already in a quote state, only the same quote ends it.

Then, inside double quotes, we need to handle backslash escapes. So when a backslash is encountered inside double quotes, check the next character. If it's one of \, $, ", or newline, then the backslash is an escape, and the next character is added literally. Otherwise, the backslash is added as is.

Wait, but how to handle newlines? For example, if there's a backslash followed by a newline inside double quotes. According to the problem statement, the backslash retains its special meaning when followed by newline. So, in that case, the backslash and newline are treated as a line continuation. But how does that affect the parsing? For example, in bash, a backslash followed by a newline in double quotes allows the command to continue on the next line, but the newline is removed. However, the current code reads the entire line from stdin using read_line, which would include the newline. So maybe the test cases don't involve multi-line input, so perhaps we can ignore handling line continuations for now. But according to the problem statement, the tests don't mention that, so perhaps we don't need to handle that case. Let me check the test examples given.

The test examples provided for double quotes are:

$ echo "quz  hello"  "bar"

quz  hello bar

$ echo "bar"  "shell's"  "foo"

bar shell's foo

So, the first test checks that double quotes preserve spaces. The second test includes a single quote inside the double-quoted string, which should be allowed. Then, the cat command has a filename with spaces and quotes inside the double quotes.

So, the main issue is correctly parsing the double quotes and handling backslash escapes for the specified characters.

So, back to the parsing function. Let's restructure it:

- Have a quote_state variable that can be None, Single, or Double.

- Iterate through each character, tracking the current state.

- When in a Double state, check for backslashes and handle escaping.

Let's outline the steps for the parse_arguments function:

1. Initialize variables: tokens vector, chars as a vector of characters, position pointer, and quote_state (None initially).

2. Skip whitespace until the next token starts.

3. For each character in the input:

a. Depending on the current quote_state:

- If in Single quotes: add characters until the next single quote.

- If in Double quotes: check for backslashes. When a backslash is found, check the next character. If it's one of \, $, ", then add the next character as a literal (skip the backslash). Else, add the backslash and the next character as literals.

- If not in quotes: check for quotes or whitespace.

4. When a closing quote is found, toggle the quote_state back to None.

Wait, but how to handle the backslash in double quotes. For example, inside double quotes, when a backslash is encountered, we need to look at the next character. If it's one of the special characters (\, $, ", or newline), then the backslash is escaping that character, so we add the next character as a literal (without the backslash). Otherwise, add the backslash and the next character as literals.

But in the code, when in double quotes, each time we see a backslash, we need to check the next character. If it's a special character, then we consume both the backslash and the next character, but only add the next character to the buffer. Otherwise, add the backslash and the next character.

Wait, for example:

Inside double quotes:

- \" becomes "

- \\ becomes \

- \$ becomes $

- \` would become ` (but that's not in the list, so \` would be treated as a literal backslash followed by a `.

But according to the problem statement, only when the backslash is followed by \, $, ", or newline does it retain its special meaning. So, in all other cases, the backslash is left as is.

So, the code for processing double quotes would be:

Inside double quote state:

loop over characters:

if current character is a backslash:

look at next character (if any)

if next character is one of \, $, ", or newline:

add the next character to the buffer

skip the backslash (i.e., don't add it, just add the next character)

increment pos by 2

else:

add the backslash to the buffer

increment pos by 1

else if current character is ", then exit double quote state

else:

add the character to the buffer and increment pos.

But wait, the backslash could be at the end of the input, so we need to check if there is a next character.

Also, handling newlines: when a backslash is followed by a newline, according to the problem statement, the backslash is an escape. In bash, this would mean that the backslash and newline are removed, allowing the command to continue on the next line. But in our case, the input is read line by line, so perhaps the newline here is part of the same input line. For example, the input might be a line like echo "hello\ there", which would be handled correctly. However, if the input line contains a backslash followed by a newline, like:

echo "hello\

world"

In bash, this would become "helloworld". But in our code, the read_line would capture up to the first newline, so the input would be "hello\ and then the next line would be world". But since the shell reads one line at a time, the first line would be processed, and the second line would be a new input. So perhaps the problem doesn't expect us to handle multi-line input, given that the shell is line-based. So maybe we can ignore the newline case for now, unless the test cases include such scenarios. Since the test examples don't mention it, perhaps we can focus on handling the other cases.

So, focusing on the backslash followed by \, $, ", or newline. Let's handle newline as part of the input line.

So, during parsing, inside double quotes:

When a backslash is encountered, check the next character. If it's one of \, $, ", then add the next character to the buffer (without the backslash). Otherwise, add the backslash and the next character as literals.

Wait, but the problem statement says that in double quotes, the backslash retains its special meaning only when followed by those characters. So, when a backslash is followed by a different character (like 'a'), the backslash is treated as a literal.

So, for example, in the input "\a" inside double quotes, it's treated as \a. But if the input is "\$", it's treated as $.

So, the code needs to:

- When in double quotes, check if the current character is a backslash.

- If yes, look ahead to the next character.

- If next character is \, $, ", or newline:

add the next character to the buffer, skip the backslash.

- else:

add the backslash and then the next character.

Wait, but newline is a special case. How is a newline represented in the input? Because when the user types a line ending with a backslash, the shell may treat it as a line continuation. But in our code, since we are using read_line, which reads until a newline, perhaps the input is processed line by line, and a backslash followed by a newline would be part of the same line. Hmm.

But in the test examples provided, there's no mention of line continuation. So perhaps the problem doesn't require handling that case, and we can focus on the other cases.

So, the code for double quotes would look like:

Inside double quote loop:

if current character is '\':

check if there's a next character.

if yes, check next character:

if it's one of '\', '$', '"', or '\n' (newline):

add the next character to buffer.

pos += 2 (skip both backslash and next char)

else:

add '\' to buffer, pos +=1

else:

add '\' to buffer, pos +=1

else if current character is '"':

exit double quote state, pos +=1

else:

add current character to buffer, pos +=1

But in Rust, the input is read via read_line, which includes the newline character as part of the string. So in the input "hello\nworld", the newline is part of the trimmed input? Wait, the code uses input.trim() to get the trimmed input. So leading and trailing whitespace is removed. But inner newlines would be part of the input? Wait, no, because when you read a line using read_line, the input is up to and including the newline. Then, trimming would remove leading and trailing whitespace, including the newline. But the newline in the middle would remain only if it's part of the input. However, in a normal terminal input, the user enters a line and hits enter, so the input is a single line. So in this scenario, the input would not contain newline characters except as the end of the line, which is trimmed. So perhaps newlines inside the input (e.g., entered via some escape) are not a concern here. But the problem statement says that backslash followed by newline is a case where the backslash is special. So perhaps the test cases could have a backslash followed by a newline within the same line.

But how would that look? For example, the user enters:

echo "hello\

world"

But the actual input line would be "hello\\world" (since the backslash is escaped?), or maybe the input line is "hello\\\nworld" ?

Wait, this is getting complicated. Since the problem statement's test examples don't include this case, maybe we can ignore it for now. Let's proceed with handling the other cases first.

So, the main changes needed are:

1. Update the parse_arguments function to handle double quotes, with backslash escaping for \, $, ", and newline.

2. Modify the tokenization logic to track whether we are in single quotes, double quotes, or not.

3. When in double quotes, process backslash escapes as per the rules.

Let me start by modifying the parse_arguments function.

First, the QuoteState enum:

enum QuoteState {

None,

Single,

Double,

}

Then, in the loop:

let mut quote_state = QuoteState::None;

Then, while pos < len:

match quote_state {

QuoteState::None => {

// check for quotes or whitespace

if c == '\'' {

quote_state = QuoteState::Single;

pos +=1;

} else if c == '"' {

quote_state = QuoteState::Double;

pos +=1;

} else if c.is_whitespace() {

break;

} else {

buffer.push(c);

pos +=1;

}

}

QuoteState::Single => {

if c == '\'' {

quote_state = QuoteState::None;

pos +=1;

} else {

buffer.push(c);

pos +=1;

}

}

QuoteState::Double => {

if c == '\\' {

// handle escape

if pos +1 < len {

let next_c = chars[pos+1];

match next_c {

'\\' | '$' | '"' | '\n' => {

buffer.push(next_c);

pos +=2;

}

_ => {

buffer.push(c); // push the backslash

pos +=1;

}

}

} else {

buffer.push(c);

pos +=1;

}

} else if c == '"' {

quote_state = QuoteState::None;

pos +=1;

} else {

buffer.push(c);

pos +=1;

}

}

}

Wait, but in the Double state, when a backslash is encountered, we need to check the next character. But what about if the next character is not present (pos+1 >= len)? Then, we just add the backslash and proceed.

So, in code:

inside QuoteState::Double:

loop over the characters, checking for backslash.

So, during the loop in the Double state:

let c = chars[pos];

if c == '\\' {

pos +=1; // move past the backslash

if pos < len {

let next_c = chars[pos];

if matches!(next_c, '\\' | '$' | '"' | '\n') {

buffer.push(next_c);

pos +=1;

} else {

buffer.push('\\');

buffer.push(next_c);

pos +=1;

}

} else {

// backslash at the end of input

buffer.push('\\');

pos +=1;

}

} else if c == '"' {

quote_state = QuoteState::None;

pos +=1;

} else {

buffer.push(c);

pos +=1;

}

Wait, perhaps this is a better way to handle it. For each character in the double quote state:

- Check if it's a backslash.

- If yes, check the next character.

- If next character is one of the special characters, add that character and skip the backslash.

- Else, add the backslash and the next character.

- If not a backslash, check if it's the closing quote.

- Else, add the character.

But handling the backslash in this way requires looking ahead.

But how to handle the case where the backslash is the last character in the input? For example, the input is "hello\".

In that case, the backslash is added as a literal.

So, in code:

Inside the double quote loop:

if pos >= len {

break;

}

let c = chars[pos];

if c == '\\' {

pos += 1; // skip the backslash

if pos < len {

let next_c = chars[pos];

if next_c == '\\' || next_c == '$' || next_c == '"' || next_c == '\n' {

buffer.push(next_c);

pos += 1;

} else {

// add the backslash and the next character

buffer.push('\\');

buffer.push(next_c);

pos += 1;

}

} else {

// backslash at the end, add it

buffer.push('\\');

}

} else if c == '"' {

// end of double quote

pos += 1;

quote_state = QuoteState::None;

} else {

buffer.push(c);

pos += 1;

}

Wait, but in this code, when we encounter a backslash, we first check if there's a next character. If yes, then check if it's one of the special characters. If so, add the next character and increment pos by 2 (since we already incremented pos by 1 when processing the backslash). Otherwise, add the backslash and the next character. However, in the code above, after incrementing pos by 1 (pos +=1 after checking c == '\\'), then the next_c is chars[pos], and then we increment pos again.

Wait, in this scenario:

Original pos is at the backslash (index i). Then pos +=1 (i+1), next_c is at i+1. Then, if we add next_c, we have to increment pos again (i+2). So total pos increases by 2, which is correct.

But in the code above, after pos +=1 (from the backslash), then check if pos < len. Then, process next_c.

For example:

Input: "a\\bc"

Inside double quotes:

At pos 0: 'a' → add to buffer, pos 1.

At pos 1: '\\' → enter backslash handling.

pos becomes 2. Check next_c at pos 2: 'b'. Since 'b' is not one of the special characters, add '\\' and 'b' to buffer. Then pos becomes 3.

Then, process 'c' → add to buffer. Then pos 4.

So the buffer would be "a\bc".

Wait, but the input was "a\\bc". So in double quotes, the code would process the two backslashes as follows:

Wait, the input is "a\\bc".

Wait, the code is handling the backslash as follows:

First, the input string after trimming would be "a\\bc" inside double quotes.

Wait, no, the actual input line would be something like echo "a\\bc".

So the input.trim() would be "echo \"a\\\\bc\"", but perhaps in the actual code, the user's input is parsed correctly. Wait, maybe I'm getting confused here. Let's think of the input as a Rust string. For example, if the user enters:

echo "hello \"world\""

The input string would be "echo \"hello \\\"world\\\"\"", but the actual code uses input.trim(), which would be "echo "hello \"world\""" (assuming the user typed it with proper escaping). But this is getting a bit complicated. Let's focus on the parsing logic.

So, the code for the double quote state:

When a backslash is encountered, check the next character. If it's one of the special characters, then the backslash is an escape, and the next character is added as a literal. Otherwise, the backslash and the next character are added as literals.

So, for example:

Input: "\\$" → would become "$".

Input: "\\a" → becomes "\a".

Input: "\$HOME" → becomes "$HOME".

But variables are not handled yet, but according to the problem statement, variables are not part of this task. So we just process the escape sequences.

So, the code should correctly handle these cases.

Now, integrating this into the parse_arguments function.

The existing code uses a while loop inside the token collection, which toggles in_quotes (for single quotes). Now, we need to replace that with a state machine that tracks single or double quotes.

So, reworking the parse_arguments function.

Let me rewrite the parse_arguments function:

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

if !buffer.is_empty() || quote_state != QuoteState::None {

tokens.push(buffer);

}

}

tokens

}

Wait, but what if the quote is not closed? For example, the user enters 'echo "hello', which is unterminated. In the current code, the buffer would collect all characters until the end, and add it to the tokens. But in a real shell, unterminated quotes would wait for more input. However, in our case, since the input is line-based, perhaps we treat the unterminated quotes as part of the token.

But according to the problem statement, the tests don't involve unterminated quotes, so maybe we can proceed under the assumption that the input is correctly quoted.

Testing the code:

Test case 1: echo "quz  hello"  "bar" → should parse into ["echo", "quz  hello", "bar"].

In the code:

- The first token is "echo".

- The next part is "quz  hello", which is inside double quotes. The code would capture all characters until the closing quote, processing any backslashes. So the buffer becomes "quz  hello".

- Then the next token is "bar".

Test case 2: echo "bar"  "shell's"  "foo" → parsed as ["echo", "bar", "shell's", "foo"]. The single quote inside double quotes is treated as a literal, so it's included.

Test case 3: cat "/tmp/file name" "/tmp/'file name' with spaces" → parsed as ["cat", "/tmp/file name", "/tmp/'file name' with spaces"]. The single quotes inside double quotes are part of the filename.

Now, handling backslashes:

For example, input: echo "hello\\world" → parsed as ["echo", "hello\world"].

Input: echo "hello\$world" → parsed as ["echo", "hello$world"], because the backslash before $ is treated as an escape.

Input: echo "hello\"world" → parsed as ["echo", "hello"world"], because the backslash before the quote escapes it.

Wait, no. Let's see: in the code, when inside double quotes, if we encounter a backslash followed by a ", then we add the " to the buffer and skip the backslash. So in the input "hello\"world", the backslash is followed by ", so the code will add the " to the buffer. So the token becomes hello"world.

So, the code should handle these cases correctly.

Now, modifying the parse_arguments function as described.

Once this function is updated, the rest of the code should work as before, since the parts vector will now correctly split the arguments even when they contain spaces inside quotes.

But need to test if the code passes the given test cases.

Now, let's see the test case where the user inputs:

$ echo "shell hello"

shell hello

In the parsed_arguments, the token after echo is "shell hello", which is passed to echo, which joins the parts with spaces. So the output is "shell hello".

Another test case:

$ echo "bar"  "shell's"  "foo" → parsed as ["echo", "bar", "shell's", "foo"], which outputs "bar shell's foo".

The cat command with "/tmp/file name" → parsed as ["cat", "/tmp/file name", ...], so the command is executed with the correct path.

Now, what about backslash in double quotes:

Input: echo "test\\" → parsed as ["echo", "test\"], since the backslash at the end is added as a literal.

But according to the problem statement, the backslash retains its special meaning only when followed by certain characters. So if the backslash is not followed by one of those characters, it's treated as a literal.

Another example: echo "hello\\nworld" → parsed as ["echo", "hello\nworld"], so the output would be hello\nworld, not a newline. But according to the problem statement, the backslash followed by newline is special, but in this case, the input has a backslash followed by 'n', which is not one of the special characters, so the backslash is treated as a literal.

So, the code should handle that correctly.

Now, the code for parse_arguments should correctly split the arguments, including handling double quotes and backslash escapes.

Now, integrating this into the existing code.

So, the main change is the parse_arguments function. The rest of the code uses the parsed parts as before.

Testing the code with the given test cases.

Another possible test case:

Input: echo "bar\"baz" → parsed as ["echo", "bar"baz"], which would output bar"baz.

But according to the code, inside double quotes, a backslash followed by a quote would escape it, so the quote is added to the buffer.

Yes. So the code should handle that.

Now, the code should now handle double quotes correctly.

So, the final code would have the modified parse_arguments function, and the rest remains the same.

But need to adjust the existing parse_arguments function to handle both single and double quotes, with the backslash processing in double quotes.

So, the answer is to modify the parse_arguments function as outlined above, which allows the code to correctly split tokens with double quotes and handle backslash escapes within them.


To solve this problem, we need to extend the shell program to handle double quotes in command arguments. Double quotes preserve the literal value of each character within them, except for backslashes, which retain their special meaning when followed by certain characters (\, $, ", or newline). This allows spaces and other characters to be included in arguments without being split by the shell.

### Approach
1. **Tokenization with Double Quotes**: Modify the input parsing to handle double quotes. This involves tokenizing the input such that any text enclosed in double quotes is treated as a single token, even if it contains spaces or other special characters.
2. **Backslash Escaping**: Process backslashes inside double quotes to handle escaping of specific characters (\, $, ", and newline). This ensures that these characters are treated correctly when preceded by a backslash.
3. **Integration with Existing Commands**: Ensure that commands like `echo` and `cat` correctly process the tokenized arguments, preserving spaces and other characters within quoted strings.

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
```

### Explanation
1. **Tokenization with Double Quotes**: The `parse_arguments` function processes the input string, splitting it into tokens while respecting both single and double quotes. This allows arguments enclosed in quotes to be treated as single tokens, including spaces and other characters.
2. **Backslash Escaping**: Inside double quotes, backslashes are processed to handle escaping of specific characters (\, $, ", and newline). This ensures that these characters are treated correctly when preceded by a backslash.
3. **Command Execution**: Commands like `echo` and `cat` use the tokenized arguments, ensuring that quoted arguments are processed correctly. The `cd` command also handles paths with spaces if they are enclosed in quotes.

This approach ensures that the shell correctly handles double quotes and backslash escapes, providing accurate and user-friendly command execution.