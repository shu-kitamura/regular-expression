// 統合テスト - CLIアプリケーションの動作をテスト

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_cli_basic_pattern_matching() {
    // テスト用ファイルを作成
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "hello world").unwrap();
    writeln!(temp_file, "foo bar").unwrap();
    writeln!(temp_file, "hello universe").unwrap();
    
    // CLIコマンドを実行
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "hello", temp_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hello world"));
    assert!(stdout.contains("hello universe"));
    assert!(!stdout.contains("foo bar"));
}

#[test]
fn test_cli_count_option() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "apple").unwrap();
    writeln!(temp_file, "banana").unwrap();
    writeln!(temp_file, "apple pie").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "-c", "apple", temp_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "2");
}

#[test]
fn test_cli_ignore_case_option() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "Hello World").unwrap();
    writeln!(temp_file, "HELLO UNIVERSE").unwrap();
    writeln!(temp_file, "goodbye").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "-i", "hello", temp_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Hello World"));
    assert!(stdout.contains("HELLO UNIVERSE"));
    assert!(!stdout.contains("goodbye"));
}

#[test]
fn test_cli_invert_match_option() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "apple").unwrap();
    writeln!(temp_file, "banana").unwrap();
    writeln!(temp_file, "cherry").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "-v", "apple", temp_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("apple"));
    assert!(stdout.contains("banana"));
    assert!(stdout.contains("cherry"));
}

#[test]
fn test_cli_line_number_option() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "first line").unwrap();
    writeln!(temp_file, "second line").unwrap();
    writeln!(temp_file, "third line").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "-n", "second", temp_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("2:second line"));
}

#[test]
fn test_cli_multiple_patterns() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "apple").unwrap();
    writeln!(temp_file, "banana").unwrap();
    writeln!(temp_file, "cherry").unwrap();
    writeln!(temp_file, "date").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "-e", "apple", "-e", "cherry", temp_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("apple"));
    assert!(stdout.contains("cherry"));
    assert!(!stdout.contains("banana"));
    assert!(!stdout.contains("date"));
}

#[test]
fn test_cli_stdin_input() {
    use std::process::{Command, Stdio};
    use std::io::Write;
    
    let mut child = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "test"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    
    // 標準入力にデータを送信
    if let Some(ref mut stdin) = child.stdin {
        writeln!(stdin, "test line").unwrap();
        writeln!(stdin, "other line").unwrap();
        writeln!(stdin, "test again").unwrap();
    }
    
    let result = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    
    assert!(stdout.contains("test line"));
    assert!(stdout.contains("test again"));
    assert!(!stdout.contains("other line"));
}

#[test]
fn test_cli_filename_options() {
    let mut temp_file1 = NamedTempFile::new().unwrap();
    let mut temp_file2 = NamedTempFile::new().unwrap();
    
    writeln!(temp_file1, "hello from file1").unwrap();
    writeln!(temp_file2, "hello from file2").unwrap();
    
    // 複数ファイルの場合、デフォルトでファイル名が表示される
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "hello", 
               temp_file1.path().to_str().unwrap(),
               temp_file2.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("{}:", temp_file1.path().to_str().unwrap())));
    assert!(stdout.contains(&format!("{}:", temp_file2.path().to_str().unwrap())));
}

#[test]
fn test_cli_no_filename_option() {
    let mut temp_file1 = NamedTempFile::new().unwrap();
    let mut temp_file2 = NamedTempFile::new().unwrap();
    
    writeln!(temp_file1, "hello from file1").unwrap();
    writeln!(temp_file2, "hello from file2").unwrap();
    
    // -h オプションでファイル名を非表示
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "-h", "hello", 
               temp_file1.path().to_str().unwrap(),
               temp_file2.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains(&format!("{}:", temp_file1.path().to_str().unwrap())));
    assert!(!stdout.contains(&format!("{}:", temp_file2.path().to_str().unwrap())));
    assert!(stdout.contains("hello from file1"));
    assert!(stdout.contains("hello from file2"));
}

#[test]
fn test_cli_error_handling() {
    // 存在しないファイルを指定
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "pattern", "nonexistent_file.txt"])
        .output()
        .expect("Failed to execute command");
    
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No such file") || stderr.contains("cannot find"));
}

#[test]
fn test_cli_no_pattern_error() {
    // パターンを指定しない場合のエラー
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--"])
        .output()
        .expect("Failed to execute command");
    
    assert!(!output.status.success());
}

#[test]
fn test_cli_conflicting_options() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "test").unwrap();
    
    // -h と -H を同時に指定
    let output = Command::new("cargo")
        .args(&["run", "--bin", "regex", "--", "-h", "-H", "test", temp_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("DuplicateFilenameOption") || stderr.contains("same time"));
}