use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    // TypeScript: src/file.ts(12,5): error TS2322: Message
    static ref TSC_ERROR: Regex = Regex::new(
        r"^(.+?)\((\d+),\d+\):\s+(?:error|warning)\s+(TS\d+):\s+(.+)$"
    ).unwrap();

    // ESLint (default formatter): file header is a path, then indented issue lines
    static ref ESLINT_FILE: Regex = Regex::new(
        r"^(/[^\s].*|[A-Za-z]:\\[^\s].*)$"
    ).unwrap();
    static ref ESLINT_ISSUE: Regex = Regex::new(
        r"^\s+(\d+):\d+\s+(?:error|warning)\s+(.+?)\s{2,}(\S+)\s*$"
    ).unwrap();

    // Cargo: error[E0308]: mismatched types
    //   --> src/main.rs:12:5
    static ref CARGO_ERROR_HEADER: Regex = Regex::new(
        r"^error\[(E\d{4})\]:\s+(.+)$"
    ).unwrap();
    static ref CARGO_LOCATION: Regex = Regex::new(
        r"^\s+--> (.+?):(\d+):\d+$"
    ).unwrap();

    // mypy: src/file.py:12: error: Message [error-code]
    static ref MYPY_ERROR: Regex = Regex::new(
        r"^(.+?):(\d+)(?::\d+)?: (?:error|warning): (.+?)\s+\[(.+)\]$"
    ).unwrap();

    // pylint: src/file.py:12:0: W0612: Unused variable 'x' (unused-variable)
    static ref PYLINT_ERROR: Regex = Regex::new(
        r"^(.+?):(\d+):\d+: ([CWER]\d{4}): (.+?) \((.+)\)$"
    ).unwrap();
}

struct ErrorGroup {
    code: String,
    message: String,
    locations: HashMap<String, Vec<usize>>,
    count: usize,
}

impl ErrorGroup {
    fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            locations: HashMap::new(),
            count: 0,
        }
    }

    fn add_location(&mut self, file: &str, line: usize) {
        self.locations
            .entry(file.to_string())
            .or_default()
            .push(line);
        self.count += 1;
    }
}

/// Groups repeated build errors by error code, preserving ALL file:line locations.
///
/// Supports: TypeScript (tsc), ESLint, Rust (cargo), Python mypy, Python pylint.
/// If no recognizable error patterns are found, returns input unchanged.
pub fn group_build_errors(input: &str) -> String {
    let mut groups: HashMap<String, ErrorGroup> = HashMap::new();
    let mut detected = false;

    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Try TSC pattern
        if let Some(caps) = TSC_ERROR.captures(line) {
            detected = true;
            let file = &caps[1];
            let line_num: usize = caps[2].parse().unwrap_or(0);
            let code = caps[3].to_string();
            let message = caps[4].to_string();

            let group = groups
                .entry(code.clone())
                .or_insert_with(|| ErrorGroup::new(&code, &message));
            group.add_location(file, line_num);
            i += 1;
            continue;
        }

        // Try Cargo pattern (two-line: error header + location)
        if let Some(caps) = CARGO_ERROR_HEADER.captures(line) {
            detected = true;
            let code = caps[1].to_string();
            let message = caps[2].to_string();

            // Look ahead for --> location line
            let mut j = i + 1;
            while j < lines.len() && j <= i + 5 {
                if let Some(loc_caps) = CARGO_LOCATION.captures(lines[j]) {
                    let file = &loc_caps[1];
                    let line_num: usize = loc_caps[2].parse().unwrap_or(0);

                    let group = groups
                        .entry(code.clone())
                        .or_insert_with(|| ErrorGroup::new(&code, &message));
                    group.add_location(file, line_num);
                    break;
                }
                j += 1;
            }
            i += 1;
            continue;
        }

        // Try mypy pattern
        if let Some(caps) = MYPY_ERROR.captures(line) {
            detected = true;
            let file = &caps[1];
            let line_num: usize = caps[2].parse().unwrap_or(0);
            let message = caps[3].to_string();
            let code = caps[4].to_string();

            let group = groups
                .entry(code.clone())
                .or_insert_with(|| ErrorGroup::new(&code, &message));
            group.add_location(file, line_num);
            i += 1;
            continue;
        }

        // Try pylint pattern
        if let Some(caps) = PYLINT_ERROR.captures(line) {
            detected = true;
            let file = &caps[1];
            let line_num: usize = caps[2].parse().unwrap_or(0);
            let code = caps[3].to_string();
            let message = caps[4].to_string();

            let group = groups
                .entry(code.clone())
                .or_insert_with(|| ErrorGroup::new(&code, &message));
            group.add_location(file, line_num);
            i += 1;
            continue;
        }

        // Try ESLint (default formatter): detect file header, then issue lines
        if ESLINT_FILE.is_match(line) {
            let file = line.trim();
            let mut j = i + 1;
            while j < lines.len() {
                if let Some(caps) = ESLINT_ISSUE.captures(lines[j]) {
                    detected = true;
                    let line_num: usize = caps[1].parse().unwrap_or(0);
                    let msg = caps[2].to_string();
                    let rule = caps[3].to_string();

                    let group = groups
                        .entry(rule.clone())
                        .or_insert_with(|| ErrorGroup::new(&rule, &msg));
                    group.add_location(file, line_num);
                    j += 1;
                } else if lines[j].trim().is_empty() {
                    j += 1;
                    break;
                } else {
                    break;
                }
            }
            i = j;
            continue;
        }

        i += 1;
    }

    // No build errors detected -> passthrough unchanged
    if !detected || groups.is_empty() {
        return input.to_string();
    }

    // Sort groups by count descending
    let mut sorted_groups: Vec<&ErrorGroup> = groups.values().collect();
    sorted_groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.code.cmp(&b.code)));

    let mut result = String::new();

    for group in &sorted_groups {
        // Header: CODE: message (xN)
        if group.count > 1 {
            result.push_str(&format!(
                "{}: {} (x{})\n",
                group.code, group.message, group.count
            ));
        } else {
            result.push_str(&format!("{}: {}\n", group.code, group.message));
        }

        // Sort files by occurrence count descending, then alphabetically
        let mut file_entries: Vec<(&String, &Vec<usize>)> = group.locations.iter().collect();
        file_entries.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(b.0)));

        // Determine max file path width for alignment
        let max_path_len = file_entries.iter().map(|(f, _)| f.len()).max().unwrap_or(0);

        // Show first 8 files inline with alignment, rest in compact form
        // SAFETY: ALL locations preserved -- never drop line numbers
        let inline_limit = 8;
        let mut shown_occurrences = 0;

        for (idx, (file, line_nums)) in file_entries.iter().enumerate() {
            let mut sorted_lines = line_nums.to_vec();
            sorted_lines.sort();
            sorted_lines.dedup();

            if idx < inline_limit {
                let lines_str: Vec<String> =
                    sorted_lines.iter().map(|n| format!(":{}", n)).collect();

                result.push_str(&format!(
                    "  {:<width$}  {}\n",
                    file,
                    lines_str.join(", "),
                    width = max_path_len
                ));
                shown_occurrences += sorted_lines.len();
            }
        }

        // Remaining files: compact but still preserve every location
        let total_files = file_entries.len();
        if total_files > inline_limit {
            let remaining_files: Vec<String> = file_entries[inline_limit..]
                .iter()
                .map(|(file, line_nums)| {
                    let mut sorted_lines = line_nums.to_vec();
                    sorted_lines.sort();
                    sorted_lines.dedup();
                    let lines_str: Vec<String> =
                        sorted_lines.iter().map(|n| format!(":{}", n)).collect();
                    format!("{}({})", file, lines_str.join(","))
                })
                .collect();
            let remaining_occurrences = group.count - shown_occurrences;
            result.push_str(&format!(
                "  ... +{} files ({} occurrences): {}\n",
                total_files - inline_limit,
                remaining_occurrences,
                remaining_files.join(", ")
            ));
        }
    }

    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: TSC errors grouped by TS code, all line numbers preserved
    #[test]
    fn test_tsc_errors_grouped_by_code() {
        let input = "\
src/api/users.ts(47,5): error TS2322: Type 'string' is not assignable to type 'number'.
src/api/users.ts(83,10): error TS2322: Type 'string' is not assignable to type 'number'.
src/api/orders.ts(12,3): error TS2322: Type 'string' is not assignable to type 'number'.
src/api/orders.ts(45,7): error TS2345: Argument of type 'number' is not assignable to parameter of type 'string'.
src/api/products.ts(23,1): error TS2322: Type 'string' is not assignable to type 'number'.
";
        let result = group_build_errors(input);
        assert!(result.contains("TS2322"));
        assert!(result.contains("TS2345"));
        assert!(result.contains(":47"));
        assert!(result.contains(":83"));
        assert!(result.contains(":12"));
        assert!(result.contains(":23"));
        assert!(result.contains(":45"));
        assert!(result.contains("(x4)"));
    }

    // Test 2: ESLint errors grouped by rule name
    #[test]
    fn test_eslint_errors_grouped_by_rule() {
        let input = "\
/Users/test/project/src/utils.ts
  10:5  error  Use const instead of let  prefer-const
  15:5  error  Use const instead of let  prefer-const

/Users/test/project/src/api.ts
  20:10  error  Variable x is unused  @typescript-eslint/no-unused-vars
  30:5   error  Use const instead of let  prefer-const

";
        let result = group_build_errors(input);
        assert!(result.contains("prefer-const"));
        assert!(result.contains("@typescript-eslint/no-unused-vars"));
        assert!(result.contains(":10"));
        assert!(result.contains(":15"));
        assert!(result.contains(":20"));
        assert!(result.contains(":30"));
        assert!(result.contains("(x3)"));
    }

    // Test 3: Cargo build errors grouped by E code
    #[test]
    fn test_cargo_errors_grouped_by_e_code() {
        let input = "\
error[E0308]: mismatched types
  --> src/main.rs:12:5
   |
12 |     let x: i32 = \"hello\";
   |                  ^^^^^^^ expected `i32`, found `&str`

error[E0308]: mismatched types
  --> src/lib.rs:45:10
   |
45 |     foo(\"bar\")
   |         ^^^^^ expected `i32`, found `&str`

error[E0599]: no method named `foo` found for struct `Bar`
  --> src/utils.rs:23:5
   |
23 |     bar.foo()
   |         ^^^ method not found
";
        let result = group_build_errors(input);
        assert!(result.contains("E0308"));
        assert!(result.contains("E0599"));
        assert!(result.contains(":12"));
        assert!(result.contains(":45"));
        assert!(result.contains(":23"));
        assert!(result.contains("(x2)"), "E0308 should appear twice");
    }

    // Test 4: ALL line numbers preserved (40 errors, verify all 40 appear)
    #[test]
    fn test_all_40_line_numbers_preserved() {
        let mut input = String::new();
        for i in 1..=40 {
            let file_idx = (i - 1) / 4 + 1;
            let line_num = i * 10;
            input.push_str(&format!(
                "src/file{}.ts({},5): error TS2322: Type 'string' is not assignable to type 'number'.\n",
                file_idx, line_num
            ));
        }
        let result = group_build_errors(&input);
        // Verify all 40 line numbers appear
        for i in 1..=40 {
            let line_num = i * 10;
            assert!(
                result.contains(&format!(":{}", line_num)),
                "Line number :{} missing from output",
                line_num
            );
        }
        assert!(result.contains("(x40)"));
    }

    // Test 5: No build errors -> passthrough unchanged
    #[test]
    fn test_no_build_errors_passthrough() {
        let input =
            "Compiling myproject v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s)\n";
        let result = group_build_errors(input);
        assert_eq!(result, input);
    }

    // Test 6: Mixed error codes -> separate groups
    #[test]
    fn test_mixed_error_codes_separate_groups() {
        let input = "\
src/a.ts(10,1): error TS2322: Type 'string' is not assignable to type 'number'.
src/b.ts(20,1): error TS2345: Argument of type 'number' is not assignable.
src/c.ts(30,1): error TS2322: Type 'string' is not assignable to type 'number'.
src/d.ts(40,1): error TS2339: Property 'x' does not exist.
";
        let result = group_build_errors(input);
        assert!(result.contains("TS2322"));
        assert!(result.contains("TS2345"));
        assert!(result.contains("TS2339"));
        assert!(result.contains("TS2322: Type 'string' is not assignable to type 'number'. (x2)"));
    }

    // Test 7: Single error -> no grouping count
    #[test]
    fn test_single_error_no_grouping_count() {
        let input =
            "src/main.ts(42,5): error TS2322: Type 'string' is not assignable to type 'number'.\n";
        let result = group_build_errors(input);
        assert!(result.contains("TS2322"));
        assert!(result.contains(":42"));
        assert!(!result.contains("(x1)"));
    }

    // Test 8: mypy errors grouped by error code
    #[test]
    fn test_mypy_errors_grouped() {
        let input = "\
src/api.py:10: error: Incompatible return value type  [return-value]
src/api.py:20: error: Incompatible return value type  [return-value]
src/models.py:30: error: Incompatible types in assignment  [assignment]
src/models.py:40: error: Name 'foo' is not defined  [name-defined]
";
        let result = group_build_errors(input);
        assert!(result.contains("return-value"));
        assert!(result.contains("assignment"));
        assert!(result.contains("name-defined"));
        assert!(result.contains(":10"));
        assert!(result.contains(":20"));
        assert!(result.contains(":30"));
        assert!(result.contains(":40"));
    }

    // Test 9: pylint errors grouped
    #[test]
    fn test_pylint_errors_grouped() {
        let input = "\
src/main.py:10:0: W0612: Unused variable 'x' (unused-variable)
src/main.py:15:4: W0612: Unused variable 'y' (unused-variable)
src/utils.py:20:0: E0602: Undefined variable 'z' (undefined-variable)
";
        let result = group_build_errors(input);
        assert!(result.contains("W0612"));
        assert!(result.contains("E0602"));
        assert!(result.contains(":10"));
        assert!(result.contains(":15"));
        assert!(result.contains(":20"));
    }

    // Test 10: Many files triggers compact summary with all locations preserved
    #[test]
    fn test_many_files_shows_summary() {
        let mut input = String::new();
        for i in 1..=12 {
            input.push_str(&format!(
                "src/file{}.ts({},5): error TS2322: Type error.\n",
                i,
                i * 10
            ));
        }
        let result = group_build_errors(&input);
        assert!(result.contains("+4 files"));
        assert!(result.contains("(x12)"));
        // All 12 line numbers must still be present
        for i in 1..=12 {
            let line_num = i * 10;
            assert!(
                result.contains(&format!(":{}", line_num)),
                "Line number :{} missing from output",
                line_num
            );
        }
    }
}
