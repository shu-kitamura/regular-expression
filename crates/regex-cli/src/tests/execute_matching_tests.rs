use crate::{Args, Regex, execute_matching};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_process_single_file() {
    // Create a temporary file with test content
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "test line 1\nno match\ntest line 2").unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();

    // Create args with the file path
    let args = Args {
        pattern: None,
        files: vec![file_path],
        patterns: vec![],
        count: false,
        ignore_case: false,
        invert_match: false,
        no_filename: false,
        with_filename: false,
        line_number: false,
        help: None,
        version: None,
    };

    // Create regex that matches "test"
    let regexes = vec![Regex::new("test", false, false).unwrap()];

    // Process the file
    let count = execute_matching(&args, &regexes);

    // Should match 2 lines
    assert_eq!(count, 2);
}

#[test]
fn test_process_multiple_files() {
    // Create temporary files
    let mut temp_file1 = NamedTempFile::new().unwrap();
    let mut temp_file2 = NamedTempFile::new().unwrap();

    writeln!(temp_file1, "test line 1\nno match").unwrap();
    writeln!(temp_file2, "test line 2\ntest line 3").unwrap();

    let file_path1 = temp_file1.path().to_str().unwrap().to_string();
    let file_path2 = temp_file2.path().to_str().unwrap().to_string();

    // Create args with file paths
    let args = Args {
        pattern: None,
        files: vec![file_path1, file_path2],
        patterns: vec![],
        count: false,
        ignore_case: false,
        invert_match: false,
        no_filename: false,
        with_filename: true,
        line_number: false,
        help: None,
        version: None,
    };

    // Create regex that matches "test"
    let regexes = vec![Regex::new("test", false, false).unwrap()];

    // Process the files
    let count = execute_matching(&args, &regexes);

    // Should match 3 lines total
    assert_eq!(count, 3);
}

#[test]
fn test_process_nonexistent_file() {
    // Create args with a nonexistent file
    let args = Args {
        pattern: None,
        files: vec!["nonexistent_file_that_should_not_exist.txt".to_string()],
        patterns: vec![],
        count: false,
        ignore_case: false,
        invert_match: false,
        no_filename: false,
        with_filename: false,
        line_number: false,
        help: None,
        version: None,
    };

    // Create regex
    let regexes = vec![Regex::new("test", false, false).unwrap()];

    // Process the file - should not panic and return 0
    let count = execute_matching(&args, &regexes);
    assert_eq!(count, 0);
}

#[test]
fn test_process_with_count_option() {
    // Create a temporary file with test content
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "apple\nbanana\napple pie\ncherry\napple tart").unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();

    // Create args with count option enabled
    let args = Args {
        pattern: None,
        files: vec![file_path],
        patterns: vec![],
        count: true, // count option enabled
        ignore_case: false,
        invert_match: false,
        no_filename: false,
        with_filename: false,
        line_number: false,
        help: None,
        version: None,
    };

    // Create regex that matches "apple"
    let regexes = vec![Regex::new("apple", false, false).unwrap()];

    // Process the file
    let count = execute_matching(&args, &regexes);

    // Should match 3 lines containing "apple"
    assert_eq!(count, 3);
}

#[test]
fn test_process_with_ignore_case() {
    // Create a temporary file with test content
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "APPLE\napple\nApple pie\nbanana").unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();

    // Create args with ignore_case option enabled
    let args = Args {
        pattern: None,
        files: vec![file_path.clone()],
        patterns: vec![],
        count: false,
        ignore_case: true, // ignore_case option enabled
        invert_match: false,
        no_filename: false,
        with_filename: false,
        line_number: false,
        help: None,
        version: None,
    };

    // Create regex that matches "apple"
    let regexes = vec![Regex::new("apple", true, false).unwrap()];

    // Process the file
    let count = execute_matching(&args, &regexes);

    // Should match 3 lines containing "apple" (case insensitive)
    assert_eq!(count, 3);

    // Now test with ignore_case disabled
    let args = Args {
        pattern: None,
        files: vec![file_path],
        patterns: vec![],
        count: false,
        ignore_case: false, // ignore_case option disabled
        invert_match: false,
        no_filename: false,
        with_filename: false,
        line_number: false,
        help: None,
        version: None,
    };

    // Create regex that matches "apple" (case sensitive)
    let regexes = vec![Regex::new("apple", false, false).unwrap()];

    // Process the file
    let count = execute_matching(&args, &regexes);

    // Should match only 1 line containing exactly "apple"
    assert_eq!(count, 1);
}

#[test]
fn test_process_with_invert_match() {
    // Create a temporary file with test content
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "apple\nbanana\norange\ngrape").unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();

    // Create args with invert_match option enabled
    let args = Args {
        pattern: None,
        files: vec![file_path],
        patterns: vec![],
        count: false,
        ignore_case: false,
        invert_match: true, // invert_match option enabled
        no_filename: false,
        with_filename: false,
        line_number: false,
        help: None,
        version: None,
    };

    // Create regex that matches "apple"
    let regexes = vec![Regex::new("apple", false, true).unwrap()];

    // Process the file
    let count = execute_matching(&args, &regexes);

    // Should match 3 lines NOT containing "apple"
    assert_eq!(count, 3);
}
