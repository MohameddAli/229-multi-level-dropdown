#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // اطبع محث shell مع مسافة
    print!("$ ");
    
    // تأكد من إظهار المحث فوراً
    io::stdout()
        .flush()
        .expect("❌ فشل في إرسال البيانات إلى الخرج");

    // احفظ إدخال المستخدم في متغير
    let mut user_command = String::new();
    
    // اقرأ الإدخال حتى ضغط Enter
    io::stdin()
        .read_line(&mut user_command)
        .expect("❌ فشل في قراءة الإدخال");

    let trimmed = user_command.trim();
    if trimmed.is_empty() {
        return; // تجاهل الإدخال الفارغ
    }

    if let Some(cmd) = trimmed.split_whitespace().next() {
        println!("{}: command not found", cmd);
    }
    
}