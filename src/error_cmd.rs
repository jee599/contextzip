//! Error stacktrace compression module.
//!
//! Detects stacktraces from 5 languages (Node.js, Python, Rust, Go, Java)
//! and compresses them by removing framework frames, keeping only user code.
//! Used as a post-processor after command-specific modules run.

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // Detection patterns
    static ref NODE_FRAME_RE: Regex = Regex::new(r"^\s+at\s+.+\(.+:\d+:\d+\)").unwrap();
    static ref NODE_FRAME_BARE_RE: Regex = Regex::new(r"^\s+at\s+.+:\d+:\d+").unwrap();
    static ref PYTHON_TRACEBACK_RE: Regex = Regex::new(r"^Traceback \(most recent call last\)").unwrap();
    static ref PYTHON_FILE_RE: Regex = Regex::new(r#"^\s+File "(.+)", line (\d+)"#).unwrap();
    static ref RUST_PANIC_RE: Regex = Regex::new(r"^thread '.*' panicked at").unwrap();
    static ref GO_GOROUTINE_RE: Regex = Regex::new(r"^goroutine \d+").unwrap();
    static ref JAVA_FRAME_RE: Regex = Regex::new(r"^\s+at\s+[\w.$]+\([\w.]+:\d+\)").unwrap();

    // Framework patterns to remove
    // Node.js
    static ref NODE_FRAMEWORK_RE: Regex = Regex::new(r"node_modules/|node:internal/").unwrap();
    // Python
    static ref PYTHON_FRAMEWORK_RE: Regex = Regex::new(r"site-packages/|/usr/lib/python|importlib|_bootstrap").unwrap();
    // Java
    static ref JAVA_FRAMEWORK_RE: Regex = Regex::new(r"java\.lang\.reflect\.|sun\.reflect\.|org\.springframework\.").unwrap();
    // Go
    static ref GO_FRAMEWORK_RE: Regex = Regex::new(r"^\s*(runtime/|runtime/debug\.|net/http\.)").unwrap();
    static ref GO_FRAME_RE: Regex = Regex::new(r"^\s+.+\.go:\d+").unwrap();
    static ref GO_FUNC_RE: Regex = Regex::new(r"^[\w./]+\(").unwrap();
    // Rust
    static ref RUST_FRAMEWORK_RE: Regex = Regex::new(r"std::rt::|tokio::runtime::|std::panicking::").unwrap();
    static ref RUST_BACKTRACE_FRAME_RE: Regex = Regex::new(r"^\s+\d+:").unwrap();

    // Extract function name and location from various frame formats
    static ref NODE_EXTRACT_RE: Regex = Regex::new(r"^\s+at\s+(?:(.+?)\s+\()?(.+):(\d+):\d+\)?").unwrap();
    static ref JAVA_EXTRACT_RE: Regex = Regex::new(r"^\s+at\s+([\w.$]+)\(([\w.]+):(\d+)\)").unwrap();
}

#[derive(Debug, PartialEq)]
enum Language {
    NodeJs,
    Python,
    Rust,
    Go,
    Java,
}

/// Detect the language of a stacktrace from the input text.
fn detect_language(input: &str) -> Option<Language> {
    for line in input.lines() {
        if PYTHON_TRACEBACK_RE.is_match(line) || PYTHON_FILE_RE.is_match(line) {
            return Some(Language::Python);
        }
        if RUST_PANIC_RE.is_match(line) {
            return Some(Language::Rust);
        }
        if GO_GOROUTINE_RE.is_match(line) {
            return Some(Language::Go);
        }
    }

    // Node.js vs Java: both use "at " prefix. Distinguish by frame format.
    for line in input.lines() {
        if JAVA_EXTRACT_RE.is_match(line) {
            // Check if it looks like Java package (com.foo.Bar or java.lang.X)
            if let Some(caps) = JAVA_EXTRACT_RE.captures(line) {
                let method = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if method.contains('.') && !method.contains('/') {
                    return Some(Language::Java);
                }
            }
        }
        if NODE_FRAME_RE.is_match(line) || NODE_FRAME_BARE_RE.is_match(line) {
            return Some(Language::NodeJs);
        }
    }

    None
}

/// Compress error stacktraces by removing framework frames and keeping user code.
/// If no stacktrace is detected, passes through unchanged (safe default).
pub fn compress_errors(input: &str) -> String {
    // First, deduplicate repeated errors
    let deduped = deduplicate_repeated_errors(input);

    let lang = detect_language(&deduped);
    match lang {
        Some(Language::NodeJs) => compress_nodejs(&deduped),
        Some(Language::Python) => compress_python(&deduped),
        Some(Language::Rust) => compress_rust(&deduped),
        Some(Language::Go) => compress_go(&deduped),
        Some(Language::Java) => compress_java(&deduped),
        None => deduped,
    }
}

/// Deduplicate repeated identical error messages.
/// Same error message appearing N times -> first occurrence + "(repeated N times)"
fn deduplicate_repeated_errors(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    if lines.len() < 2 {
        return input.to_string();
    }

    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let current = lines[i];
        let mut count = 1;

        // Count consecutive identical lines
        while i + count < lines.len() && lines[i + count] == current {
            count += 1;
        }

        result.push(current.to_string());
        if count > 1 {
            result.push(format!("  (repeated {} times)", count));
        }
        i += count;
    }

    result.join("\n")
}

fn is_node_framework_frame(line: &str) -> bool {
    NODE_FRAMEWORK_RE.is_match(line)
}

fn compress_nodejs(input: &str) -> String {
    let mut result = Vec::new();
    let mut hidden_count = 0;

    for line in input.lines() {
        let is_frame = NODE_FRAME_RE.is_match(line) || NODE_FRAME_BARE_RE.is_match(line);

        if is_frame && is_node_framework_frame(line) {
            hidden_count += 1;
            continue;
        }

        if is_frame {
            // User code frame — format it nicely
            if let Some(caps) = NODE_EXTRACT_RE.captures(line) {
                let func = caps.get(1).map(|m| m.as_str()).unwrap_or("<anonymous>");
                let file = caps.get(2).map(|m| m.as_str()).unwrap_or("?");
                let line_num = caps.get(3).map(|m| m.as_str()).unwrap_or("?");
                result.push(format!(
                    "  → {}:{}         {}()",
                    file.trim(),
                    line_num,
                    func.trim()
                ));
            } else {
                result.push(format!("  → {}", line.trim()));
            }
        } else {
            // Flush hidden count before non-frame lines
            if hidden_count > 0 {
                result.push(format!("  (+ {} framework frames hidden)", hidden_count));
                hidden_count = 0;
            }
            result.push(line.to_string());
        }
    }

    if hidden_count > 0 {
        result.push(format!("  (+ {} framework frames hidden)", hidden_count));
    }

    result.join("\n")
}

fn compress_python(input: &str) -> String {
    let mut result = Vec::new();
    let mut hidden_count = 0;
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if let Some(caps) = PYTHON_FILE_RE.captures(line) {
            let file = caps.get(1).map(|m| m.as_str()).unwrap_or("?");
            let line_num = caps.get(2).map(|m| m.as_str()).unwrap_or("?");

            if PYTHON_FRAMEWORK_RE.is_match(file) {
                hidden_count += 1;
                // Skip the next line too (it's the code line)
                i += 1;
                if i < lines.len()
                    && !PYTHON_FILE_RE.is_match(lines[i])
                    && !lines[i].starts_with("Traceback")
                {
                    i += 1;
                }
                continue;
            }

            // User code frame
            // Get the code line (next line)
            let code = if i + 1 < lines.len()
                && !PYTHON_FILE_RE.is_match(lines[i + 1])
                && !lines[i + 1].starts_with("Traceback")
            {
                i += 1;
                lines[i].trim()
            } else {
                ""
            };

            if code.is_empty() {
                result.push(format!("  → {}:{}", file, line_num));
            } else {
                result.push(format!("  → {}:{}         {}", file, line_num, code));
            }
        } else {
            if hidden_count > 0 {
                result.push(format!("  (+ {} framework frames hidden)", hidden_count));
                hidden_count = 0;
            }
            result.push(line.to_string());
        }

        i += 1;
    }

    if hidden_count > 0 {
        result.push(format!("  (+ {} framework frames hidden)", hidden_count));
    }

    result.join("\n")
}

fn compress_rust(input: &str) -> String {
    let mut result = Vec::new();
    let mut hidden_count = 0;

    for line in input.lines() {
        if RUST_BACKTRACE_FRAME_RE.is_match(line) {
            if RUST_FRAMEWORK_RE.is_match(line) {
                hidden_count += 1;
                continue;
            }
            // User code frame
            result.push(format!("  → {}", line.trim()));
        } else {
            if hidden_count > 0 {
                result.push(format!("  (+ {} framework frames hidden)", hidden_count));
                hidden_count = 0;
            }
            result.push(line.to_string());
        }
    }

    if hidden_count > 0 {
        result.push(format!("  (+ {} framework frames hidden)", hidden_count));
    }

    result.join("\n")
}

fn compress_go(input: &str) -> String {
    let mut result = Vec::new();
    let mut hidden_count = 0;

    for line in input.lines() {
        // Go frames alternate: function line, then file:line line
        let is_go_file_frame = GO_FRAME_RE.is_match(line);
        let is_go_func = GO_FUNC_RE.is_match(line.trim());

        if (is_go_file_frame || is_go_func) && GO_FRAMEWORK_RE.is_match(line) {
            hidden_count += 1;
            continue;
        }

        if is_go_file_frame || is_go_func {
            // User code
            result.push(format!("  → {}", line.trim()));
        } else {
            if hidden_count > 0 {
                result.push(format!("  (+ {} framework frames hidden)", hidden_count));
                hidden_count = 0;
            }
            result.push(line.to_string());
        }
    }

    if hidden_count > 0 {
        result.push(format!("  (+ {} framework frames hidden)", hidden_count));
    }

    result.join("\n")
}

fn compress_java(input: &str) -> String {
    let mut result = Vec::new();
    let mut hidden_count = 0;

    for line in input.lines() {
        if JAVA_EXTRACT_RE.is_match(line) {
            if JAVA_FRAMEWORK_RE.is_match(line) {
                hidden_count += 1;
                continue;
            }

            // User code frame
            if let Some(caps) = JAVA_EXTRACT_RE.captures(line) {
                let method = caps.get(1).map(|m| m.as_str()).unwrap_or("?");
                let file = caps.get(2).map(|m| m.as_str()).unwrap_or("?");
                let line_num = caps.get(3).map(|m| m.as_str()).unwrap_or("?");
                result.push(format!("  → {}:{}         {}()", file, line_num, method));
            } else {
                result.push(format!("  → {}", line.trim()));
            }
        } else {
            if hidden_count > 0 {
                result.push(format!("  (+ {} framework frames hidden)", hidden_count));
                hidden_count = 0;
            }
            result.push(line.to_string());
        }
    }

    if hidden_count > 0 {
        result.push(format!("  (+ {} framework frames hidden)", hidden_count));
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nodejs_stacktrace() {
        let input = r#"TypeError: Cannot read properties of undefined (reading 'id')
    at getUserProfile (/app/src/api/users.ts:47:12)
    at processAuth (/app/src/middleware/auth.ts:12:5)
    at Module._compile (node:internal/modules/cjs/loader:1376:14)
    at Object.Module._extensions..js (node:internal/modules/cjs/loader:1435:10)
    at Module.load (node:internal/modules/cjs/loader:1207:32)
    at Function.Module._load (node:internal/modules/cjs/loader:1023:12)
    at Router.handle (/app/node_modules/express/lib/router/index.js:45:12)
    at Layer.handle (/app/node_modules/express/lib/router/layer.js:95:5)"#;

        let result = compress_errors(input);

        // Must preserve error message
        assert!(result.contains("TypeError: Cannot read properties of undefined"));
        // Must keep user code frames
        assert!(result.contains("src/api/users.ts:47"));
        assert!(result.contains("src/middleware/auth.ts:12"));
        // Must remove framework frames
        assert!(!result.contains("node:internal"));
        assert!(!result.contains("node_modules"));
        // Must show hidden count
        assert!(result.contains("framework frames hidden"));
    }

    #[test]
    fn test_python_traceback() {
        let input = r#"Traceback (most recent call last):
  File "/app/src/handler.py", line 42, in process_request
    result = compute(data)
  File "/usr/lib/python3.11/importlib/__init__.py", line 126, in import_module
    return _bootstrap._gcd_import(name[level:], package, level)
  File "/app/venv/lib/python3.11/site-packages/flask/app.py", line 1498, in __call__
    return self.wsgi_app(environ, start_response)
  File "/app/src/utils.py", line 18, in compute
    return x / y
ZeroDivisionError: division by zero"#;

        let result = compress_errors(input);

        // Must preserve error message
        assert!(result.contains("ZeroDivisionError: division by zero"));
        // Must keep user code frames
        assert!(result.contains("src/handler.py:42"));
        assert!(result.contains("src/utils.py:18"));
        // Must remove framework frames
        assert!(!result.contains("site-packages"));
        assert!(!result.contains("importlib"));
        // Must show hidden count
        assert!(result.contains("framework frames hidden"));
    }

    #[test]
    fn test_rust_panic() {
        let input = r#"thread 'main' panicked at 'index out of bounds: the len is 3 but the index is 5', src/main.rs:42:10
stack backtrace:
   0: std::panicking::begin_panic_handler
   1: core::panicking::panic_fmt
   2: core::panicking::panic_bounds_check
   3: myapp::process_data
   4: myapp::main
   5: std::rt::lang_start
   6: std::rt::lang_start::{{closure}}"#;

        let result = compress_errors(input);

        // Must preserve panic message
        assert!(result.contains("panicked at"));
        assert!(result.contains("index out of bounds"));
        // Must keep user code frames
        assert!(result.contains("myapp::process_data"));
        assert!(result.contains("myapp::main"));
        // Must remove framework frames
        assert!(!result.contains("std::panicking::begin_panic_handler"));
        assert!(!result.contains("std::rt::lang_start"));
        // Must show hidden count
        assert!(result.contains("framework frames hidden"));
    }

    #[test]
    fn test_go_goroutine() {
        let input = r#"goroutine 1 [running]:
runtime/debug.Stack()
	/usr/local/go/src/runtime/debug/stack.go:24 +0x5e
runtime/debug.PrintStack()
	/usr/local/go/src/runtime/debug/stack.go:16 +0x1a
main.handleRequest()
	/app/src/handler.go:35 +0x2a
main.main()
	/app/src/main.go:12 +0x1f"#;

        let result = compress_errors(input);

        // Must preserve goroutine header
        assert!(result.contains("goroutine 1 [running]"));
        // Must keep user code frames
        assert!(result.contains("handler.go:35"));
        assert!(result.contains("main.go:12"));
        // Must remove runtime frames
        assert!(!result.contains("runtime/debug.Stack"));
        assert!(!result.contains("runtime/debug.PrintStack"));
        // Must show hidden count
        assert!(result.contains("framework frames hidden"));
    }

    #[test]
    fn test_java_stacktrace() {
        let input = r#"java.lang.NullPointerException: Cannot invoke method on null
	at com.myapp.service.UserService.getUser(UserService.java:42)
	at com.myapp.controller.UserController.show(UserController.java:18)
	at java.lang.reflect.Method.invoke(Method.java:566)
	at sun.reflect.NativeMethodAccessorImpl.invoke(NativeMethodAccessorImpl.java:62)
	at org.springframework.web.servlet.FrameworkServlet.service(FrameworkServlet.java:897)"#;

        let result = compress_errors(input);

        // Must preserve error message
        assert!(result.contains("NullPointerException"));
        // Must keep user code frames
        assert!(result.contains("UserService.java:42"));
        assert!(result.contains("UserController.java:18"));
        // Must remove framework frames
        assert!(!result.contains("java.lang.reflect.Method"));
        assert!(!result.contains("sun.reflect"));
        assert!(!result.contains("org.springframework"));
        // Must show hidden count
        assert!(result.contains("framework frames hidden"));
    }

    #[test]
    fn test_repeated_errors() {
        let input = "Error: connection refused\nError: connection refused\nError: connection refused\nError: connection refused";

        let result = compress_errors(input);

        // Should contain the error once
        assert!(result.contains("Error: connection refused"));
        // Should show repeat count
        assert!(result.contains("repeated 4 times"));
    }

    #[test]
    fn test_no_stacktrace_passthrough() {
        let input = "Building project...\nCompiling 42 modules\nDone in 3.2s";

        let result = compress_errors(input);

        // Should pass through unchanged
        assert_eq!(result, input);
    }

    #[test]
    fn test_mixed_content_with_stacktrace() {
        let input = r#"Server starting on port 3000
Handling request GET /api/users
TypeError: Cannot read properties of undefined (reading 'name')
    at getUser (/app/src/routes/users.ts:23:15)
    at Layer.handle (/app/node_modules/express/lib/router/layer.js:95:5)
    at Router.handle (/app/node_modules/express/lib/router/index.js:45:12)
    at Object.Module._extensions..js (node:internal/modules/cjs/loader:1435:10)
Request failed"#;

        let result = compress_errors(input);

        // Must preserve non-stacktrace content
        assert!(result.contains("Server starting on port 3000"));
        assert!(result.contains("Handling request"));
        assert!(result.contains("Request failed"));
        // Must preserve error and user frames
        assert!(result.contains("TypeError"));
        assert!(result.contains("src/routes/users.ts:23"));
        // Must remove framework frames
        assert!(!result.contains("node_modules"));
        assert!(!result.contains("node:internal"));
    }

    #[test]
    fn test_detect_language_nodejs() {
        let input = "Error\n    at foo (/app/src/index.ts:10:5)";
        assert_eq!(detect_language(input), Some(Language::NodeJs));
    }

    #[test]
    fn test_detect_language_python() {
        let input = "Traceback (most recent call last):\n  File \"test.py\", line 1";
        assert_eq!(detect_language(input), Some(Language::Python));
    }

    #[test]
    fn test_detect_language_rust() {
        let input = "thread 'main' panicked at 'error', src/main.rs:1:1";
        assert_eq!(detect_language(input), Some(Language::Rust));
    }

    #[test]
    fn test_detect_language_go() {
        let input = "goroutine 1 [running]:";
        assert_eq!(detect_language(input), Some(Language::Go));
    }

    #[test]
    fn test_detect_language_java() {
        let input = "\tat com.example.Main.run(Main.java:10)";
        assert_eq!(detect_language(input), Some(Language::Java));
    }

    #[test]
    fn test_detect_language_none() {
        let input = "just some normal text";
        assert_eq!(detect_language(input), None);
    }
}
