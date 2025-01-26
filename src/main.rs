use std::collections::HashSet;
use std::io::{self, Write};
use std::process;

fn main() {
    // قائمة بالأوامر المدمجة في الـ shell
    let builtins: HashSet<&str> = ["exit", "echo", "type"].iter().cloned().collect();

    loop {
        // عرض الـ prompt
        print!("$ ");
        io::stdout().flush().expect("فشل في إرسال البيانات");

        // قراءة الإدخال
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // إنهاء عند الضغط على Ctrl+D
            Ok(_) => {}
            Err(e) => {
                eprintln!("خطأ: {}", e);
                break;
            }
        }

        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue; // تجاهل الإدخال الفارغ
        }

        // تقسيم الإدخال إلى أجزاء
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let command = parts[0];

        // معالجة الأوامر
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
                    continue; // تجاهل إذا لم يُحدد أمر للفحص
                }
                let cmd_to_check = parts[1];
                if builtins.contains(cmd_to_check) {
                    println!("{} is a shell builtin", cmd_to_check);
                } else {
                    println!("{}: not found", cmd_to_check);
                }
            }
            _ => println!("{}: command not found", command),
        }
    }
}