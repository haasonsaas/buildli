use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

pub fn print_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

pub fn format_code_snippet(content: &str, file_path: &str, line_start: usize) -> String {
    let mut result = String::new();
    
    result.push_str(&format!("\n{}\n", file_path.cyan()));
    result.push_str(&"─".repeat(file_path.len()).dimmed().to_string());
    result.push('\n');
    
    for (i, line) in content.lines().enumerate() {
        let line_num = line_start + i;
        result.push_str(&format!(
            "{} {}\n",
            format!("{:4}", line_num).dimmed(),
            line
        ));
    }
    
    result
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }
}