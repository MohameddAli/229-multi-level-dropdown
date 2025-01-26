use std::io::{self, Write};
use std::process;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // إنهاء عند الضغط على Ctrl+D
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
                // أخذ جميع الأجزاء بعد "echo" ودمجها بمسافات
                let output = parts[1..].join(" ");
                println!("{}", output);
            }
            _ => println!("{}: command not found", command),
        }
    }
}