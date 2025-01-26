#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        // طباعة الـ prompt في كل تكرار
        print!("$ ");
        io::stdout().flush().expect("فشل في إفراز البافر");

        // قراءة الإدخال من المستخدم
        let mut user_command = String::new();
        match io::stdin().read_line(&mut user_command) {
            Ok(0) => break,    // حالة نهاية الإدخال (Ctrl+D)
            Ok(_) => {},       // قراءة ناجحة
            Err(e) => {        // معالجة الأخطاء
                eprintln!("خطأ في القراءة: {}", e);
                break;
            }
        }

        // تجاهل الإدخال الفارغ أو المسافات فقط
        let trimmed_user_command = user_command.trim();
        if trimmed_user_command.is_empty() {
            continue;
        }

        // استخراج اسم الأمر الأول
        if let Some(command_name) = trimmed_user_command.split_whitespace().next() {
            println!("{}: command not found", command_name);
        }
    }
}