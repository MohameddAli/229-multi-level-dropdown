#[allow(unused_imports)]
use std::io::{self, Write};
use std::process;

fn main() {
    loop {
        // 1. عرض الـ prompt
        print!("$ ");
        io::stdout().flush().expect("فشل في إرسال البيانات");

        // 2. قراءة الإدخال
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

        // 3. تقسيم الإدخال إلى أجزاء
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let command = parts[0];

        // 4. التحقق من أمر exit
        if command == "exit" {
            let exit_code = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            process::exit(exit_code); // إنهاء البرنامج بالكود
        } else {
            println!("{}: command not found", command);
        }
    }
}