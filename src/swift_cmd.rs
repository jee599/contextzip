use crate::tracking;
use crate::utils::{resolved_command, truncate};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::ffi::OsString;

lazy_static! {
    // Xcode build errors: /path/to/file.swift:42:15: error: message
    static ref SWIFT_ERROR_RE: Regex = Regex::new(
        r"^(.+\.swift):(\d+):\d+:\s+(error|warning|note):\s+(.+)$"
    ).unwrap();

    // CompileSwift noise lines
    static ref COMPILE_SWIFT_RE: Regex = Regex::new(
        r"^CompileSwift\s+normal\s+\S+\s+(.+)$"
    ).unwrap();

    // CompileC noise lines
    static ref COMPILE_C_RE: Regex = Regex::new(
        r"^CompileC\s+"
    ).unwrap();

    // Linking noise
    static ref LINK_RE: Regex = Regex::new(
        r"^Ld\s+|^Linking\s+"
    ).unwrap();

    // MergeSwiftModule, EmitSwiftModule, etc.
    static ref MODULE_RE: Regex = Regex::new(
        r"^(?:MergeSwiftModule|EmitSwiftModule|SwiftMergeGeneratedHeaders|SwiftDriver|SwiftCompile)\s+"
    ).unwrap();

    // Build timestamp lines like "2024-01-15 10:30:00.000 xcodebuild[12345:67890]"
    static ref XCODE_TIMESTAMP_RE: Regex = Regex::new(
        r"^\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d+\s+xcodebuild\["
    ).unwrap();

    // ProcessInfoPlistFile, CopySwiftLibs, CodeSign, etc.
    static ref BUILD_STEP_RE: Regex = Regex::new(
        r"^(?:ProcessInfoPlistFile|CopySwiftLibs|CodeSign|CreateBuildDirectory|RegisterWithLaunchServices|Validate|Touch|PhaseScriptExecution|WriteAuxiliaryFile|ProcessProductPackaging|GenerateDSYMFile|Strip)\s+"
    ).unwrap();

    // Empty cd/setenv/export commands in verbose xcodebuild output
    static ref VERBOSE_CMD_RE: Regex = Regex::new(
        r"^\s+(?:cd|setenv|export)\s+"
    ).unwrap();

    // "Build settings from command line:" and the key=value lines that follow
    static ref BUILD_SETTINGS_RE: Regex = Regex::new(
        r"^(?:Build settings from |    \w+\s*=\s*)"
    ).unwrap();

    // Swift crash stack trace frame: "0   libswiftCore.dylib  0x00007fff... symbol + offset"
    static ref CRASH_FRAME_RE: Regex = Regex::new(
        r"^\d+\s+(\S+)\s+0x[0-9a-fA-F]+\s+(.+)$"
    ).unwrap();

    // Framework dylibs to collapse in crash traces
    static ref FRAMEWORK_DYLIB_RE: Regex = Regex::new(
        r"^(?:libswift|UIKit|Foundation|CoreFoundation|libdispatch|libsystem|libobjc|GraphicsServices|CoreGraphics|QuartzCore|CFNetwork|Security|libnetwork|AppleMetalOpenGLRenderer)"
    ).unwrap();

    // "** BUILD SUCCEEDED **" or "** BUILD FAILED **"
    static ref BUILD_RESULT_RE: Regex = Regex::new(
        r"^\*\* BUILD (?:SUCCEEDED|FAILED) \*\*"
    ).unwrap();

    // Xcode "note: ..." lines that aren't attached to errors
    static ref NOTE_LINE_RE: Regex = Regex::new(
        r"^note:\s+"
    ).unwrap();

    // "=== BUILD TARGET ... ===" separator
    static ref BUILD_TARGET_RE: Regex = Regex::new(
        r"^===\s+BUILD\s+"
    ).unwrap();
}

#[derive(Debug, Clone)]
pub enum SwiftCommand {
    Build,
    Test,
}

pub fn run(cmd: SwiftCommand, args: &[String], verbose: u8) -> Result<()> {
    match cmd {
        SwiftCommand::Build => run_build(args, verbose),
        SwiftCommand::Test => run_test(args, verbose),
    }
}

pub fn run_passthrough(args: &[OsString], verbose: u8) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("swift: no subcommand specified");
    }

    let timer = tracking::TimedExecution::start();
    let subcommand = args[0].to_string_lossy().to_string();

    let mut cmd = resolved_command("swift");
    cmd.arg(&subcommand);
    for arg in &args[1..] {
        cmd.arg(arg);
    }

    if verbose > 0 {
        eprintln!("Running: swift {} ...", subcommand);
    }

    let output = cmd
        .output()
        .with_context(|| format!("Failed to run swift {}. Is Swift installed?", subcommand))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = format!("{}\n{}", stdout, stderr);

    let exit_code = output
        .status
        .code()
        .unwrap_or(if output.status.success() { 0 } else { 1 });
    let filtered = truncate(&raw, 500);

    if let Some(hint) = crate::tee::tee_and_hint(&raw, &format!("swift_{}", subcommand), exit_code)
    {
        println!("{}\n{}", filtered, hint);
    } else {
        println!("{}", filtered);
    }

    timer.track(
        &format!("swift {}", subcommand),
        &format!("contextzip swift {}", subcommand),
        &raw,
        &filtered,
    );

    if !output.status.success() {
        std::process::exit(exit_code);
    }

    Ok(())
}

/// Generic swift command runner with filtering
fn run_swift_filtered<F>(
    subcommand: &str,
    args: &[String],
    verbose: u8,
    filter_fn: F,
) -> Result<()>
where
    F: Fn(&str) -> String,
{
    let timer = tracking::TimedExecution::start();

    let mut cmd = resolved_command("swift");
    cmd.arg(subcommand);

    for arg in args {
        cmd.arg(arg);
    }

    if verbose > 0 {
        eprintln!("Running: swift {} {}", subcommand, args.join(" "));
    }

    let output = cmd
        .output()
        .with_context(|| format!("Failed to run swift {}", subcommand))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = format!("{}\n{}", stdout, stderr);

    let exit_code = output
        .status
        .code()
        .unwrap_or(if output.status.success() { 0 } else { 1 });
    let filtered = filter_fn(&raw);

    if let Some(hint) = crate::tee::tee_and_hint(&raw, &format!("swift_{}", subcommand), exit_code)
    {
        println!("{}\n{}", filtered, hint);
    } else {
        println!("{}", filtered);
    }

    timer.track(
        &format!("swift {} {}", subcommand, args.join(" ")),
        &format!("contextzip swift {} {}", subcommand, args.join(" ")),
        &raw,
        &filtered,
    );

    if !output.status.success() {
        std::process::exit(exit_code);
    }

    Ok(())
}

fn run_build(args: &[String], verbose: u8) -> Result<()> {
    run_swift_filtered("build", args, verbose, filter_swift_build)
}

fn run_test(args: &[String], verbose: u8) -> Result<()> {
    run_swift_filtered("test", args, verbose, filter_swift_test)
}

/// Filter swift build output: collapse CompileSwift lines, keep errors/warnings.
pub fn filter_swift_build(output: &str) -> String {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut compile_count = 0;
    let mut compile_c_count = 0;
    let mut link_count = 0;
    let mut other_steps = 0;
    let mut in_error_block = false;
    let mut current_block: Vec<String> = Vec::new();
    let mut current_severity = "";
    let mut build_result = String::new();

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if in_error_block && !current_block.is_empty() {
                match current_severity {
                    "error" => errors.push(current_block.join("\n")),
                    _ => warnings.push(current_block.join("\n")),
                }
                current_block.clear();
                in_error_block = false;
                current_severity = "";
            }
            continue;
        }

        // Count and collapse CompileSwift lines
        if COMPILE_SWIFT_RE.is_match(trimmed) {
            compile_count += 1;
            continue;
        }

        // Count CompileC lines
        if COMPILE_C_RE.is_match(trimmed) {
            compile_c_count += 1;
            continue;
        }

        // Count link lines
        if LINK_RE.is_match(trimmed) {
            link_count += 1;
            continue;
        }

        // Skip module/build step noise
        if MODULE_RE.is_match(trimmed)
            || BUILD_STEP_RE.is_match(trimmed)
            || XCODE_TIMESTAMP_RE.is_match(trimmed)
            || VERBOSE_CMD_RE.is_match(trimmed)
            || BUILD_SETTINGS_RE.is_match(trimmed)
        {
            other_steps += 1;
            continue;
        }

        // Capture build result
        if BUILD_RESULT_RE.is_match(trimmed) {
            build_result = trimmed.to_string();
            continue;
        }

        // Skip standalone note lines
        if NOTE_LINE_RE.is_match(trimmed) && !in_error_block {
            continue;
        }

        // Skip build target separators
        if BUILD_TARGET_RE.is_match(trimmed) {
            continue;
        }

        // Detect error/warning lines
        if let Some(caps) = SWIFT_ERROR_RE.captures(trimmed) {
            // Flush previous block
            if in_error_block && !current_block.is_empty() {
                match current_severity {
                    "error" => errors.push(current_block.join("\n")),
                    _ => warnings.push(current_block.join("\n")),
                }
                current_block.clear();
            }

            let severity = caps.get(3).map(|m| m.as_str()).unwrap_or("error");
            in_error_block = true;
            current_severity = if severity == "error" { "error" } else { "warning" };
            current_block.push(line.to_string());
            continue;
        }

        // Continuation of error block (context lines, caret markers, etc.)
        if in_error_block {
            current_block.push(line.to_string());
            continue;
        }
    }

    // Flush final block
    if in_error_block && !current_block.is_empty() {
        match current_severity {
            "error" => errors.push(current_block.join("\n")),
            _ => warnings.push(current_block.join("\n")),
        }
    }

    let mut result = Vec::new();

    // Summary line
    let mut parts = Vec::new();
    if compile_count > 0 {
        parts.push(format!(
            "{} Swift file{} compiled",
            compile_count,
            if compile_count > 1 { "s" } else { "" }
        ));
    }
    if compile_c_count > 0 {
        parts.push(format!(
            "{} C file{} compiled",
            compile_c_count,
            if compile_c_count > 1 { "s" } else { "" }
        ));
    }
    if link_count > 0 {
        parts.push(format!(
            "{} target{} linked",
            link_count,
            if link_count > 1 { "s" } else { "" }
        ));
    }
    if other_steps > 0 {
        parts.push(format!("{} build steps", other_steps));
    }

    if !parts.is_empty() {
        result.push(format!("swift build: {}", parts.join(", ")));
    }

    // Errors
    if !errors.is_empty() {
        result.push(format!(
            "\n{} error{}:",
            errors.len(),
            if errors.len() > 1 { "s" } else { "" }
        ));
        for err in &errors {
            result.push(err.clone());
        }
    }

    // Warnings (show up to 5)
    if !warnings.is_empty() {
        let shown = warnings.len().min(5);
        result.push(format!(
            "\n{} warning{}{}:",
            warnings.len(),
            if warnings.len() > 1 { "s" } else { "" },
            if warnings.len() > shown {
                format!(", showing {}", shown)
            } else {
                String::new()
            }
        ));
        for warn in warnings.iter().take(shown) {
            result.push(warn.clone());
        }
    }

    // Build result
    if !build_result.is_empty() {
        result.push(String::new());
        result.push(build_result);
    }

    if result.is_empty() {
        return output.to_string();
    }

    result.join("\n")
}

/// Filter swift test output: keep test results and failures, strip compilation noise.
fn filter_swift_test(output: &str) -> String {
    // First pass: filter build noise
    let build_filtered = filter_swift_build(output);

    // The test output follows the build output, so return as-is for now
    // Swift test output is already fairly compact
    build_filtered
}

/// Compress Swift crash stack traces by collapsing framework frames.
/// Called from error_cmd as part of the stack trace compression pipeline.
pub fn compress_swift_crash(input: &str) -> String {
    let mut result = Vec::new();
    let mut hidden_count: usize = 0;

    for line in input.lines() {
        if let Some(caps) = CRASH_FRAME_RE.captures(line) {
            let dylib = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let symbol = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            if FRAMEWORK_DYLIB_RE.is_match(dylib) {
                hidden_count += 1;
                continue;
            }

            // User/app frame — keep it
            if hidden_count > 0 {
                result.push(format!(
                    "  (+ {} framework frames hidden)",
                    hidden_count
                ));
                hidden_count = 0;
            }
            result.push(format!("  → {} {}", dylib, symbol.trim()));
        } else {
            // Non-frame line
            if hidden_count > 0 {
                result.push(format!(
                    "  (+ {} framework frames hidden)",
                    hidden_count
                ));
                hidden_count = 0;
            }
            result.push(line.to_string());
        }
    }

    if hidden_count > 0 {
        result.push(format!(
            "  (+ {} framework frames hidden)",
            hidden_count
        ));
    }

    result.join("\n")
}

/// Compress Xcode build log by collapsing repetitive compilation lines.
/// Can be used as a post-processor like docker_cmd::compress_docker_log.
pub fn compress_xcode_log(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    if lines.len() < 5 {
        return input.to_string();
    }

    let mut result = Vec::new();
    let mut compile_swift_count = 0;
    let mut compile_c_count = 0;
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        if COMPILE_SWIFT_RE.is_match(trimmed) {
            compile_swift_count += 1;
            i += 1;
            continue;
        }

        if COMPILE_C_RE.is_match(trimmed) {
            compile_c_count += 1;
            i += 1;
            continue;
        }

        // Flush compile counts before other lines
        if compile_swift_count > 0 {
            result.push(format!("CompileSwift: {} files compiled", compile_swift_count));
            compile_swift_count = 0;
        }
        if compile_c_count > 0 {
            result.push(format!("CompileC: {} files compiled", compile_c_count));
            compile_c_count = 0;
        }

        result.push(lines[i].to_string());
        i += 1;
    }

    // Flush remaining counts
    if compile_swift_count > 0 {
        result.push(format!("CompileSwift: {} files compiled", compile_swift_count));
    }
    if compile_c_count > 0 {
        result.push(format!("CompileC: {} files compiled", compile_c_count));
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_swift_build_compiles() {
        let input = r#"CompileSwift normal arm64 /Users/dev/project/Sources/App.swift
CompileSwift normal arm64 /Users/dev/project/Sources/Models/User.swift
CompileSwift normal arm64 /Users/dev/project/Sources/Views/ContentView.swift
Ld /Users/dev/build/Debug/MyApp normal arm64
** BUILD SUCCEEDED **"#;

        let result = filter_swift_build(input);
        assert!(
            result.contains("3 Swift files compiled"),
            "Should count compile lines: {}",
            result
        );
        assert!(
            result.contains("1 target linked"),
            "Should count link lines: {}",
            result
        );
        assert!(
            result.contains("BUILD SUCCEEDED"),
            "Should keep build result: {}",
            result
        );
        assert!(
            !result.contains("CompileSwift normal"),
            "Should strip individual CompileSwift lines: {}",
            result
        );
    }

    #[test]
    fn test_filter_swift_build_errors_preserved() {
        let input = r#"CompileSwift normal arm64 /Users/dev/project/Sources/App.swift
CompileSwift normal arm64 /Users/dev/project/Sources/Models/User.swift
/Users/dev/project/Sources/App.swift:42:15: error: cannot convert value of type 'String' to expected argument type 'Int'
        let x: Int = name
                     ^~~~
/Users/dev/project/Sources/Models/User.swift:10:5: warning: variable 'unused' was never used
        let unused = 42
            ^~~~~~
** BUILD FAILED **"#;

        let result = filter_swift_build(input);
        assert!(
            result.contains("cannot convert value"),
            "Should preserve error message: {}",
            result
        );
        assert!(
            result.contains("App.swift:42:15"),
            "Should preserve error location: {}",
            result
        );
        assert!(
            result.contains("variable 'unused' was never used"),
            "Should preserve warning: {}",
            result
        );
        assert!(
            result.contains("BUILD FAILED"),
            "Should keep build result: {}",
            result
        );
        assert!(
            !result.contains("CompileSwift normal"),
            "Should strip CompileSwift lines: {}",
            result
        );
    }

    #[test]
    fn test_compress_swift_crash_trace() {
        let input = r#"Thread 0 Crashed:
0   libswiftCore.dylib  0x00007fff2040a123 _swift_runtime_on_report + 123
1   libswiftCore.dylib  0x00007fff2040b456 _swift_stdlib_reportFatalError + 56
2   MyApp               0x000000010a234567 MyApp.ViewController.viewDidLoad() -> () + 234
3   UIKitCore           0x00007fff23456789 -[UIViewController _sendViewDidLoadWithAppearanceProxyObjectTaggingEnabled] + 100
4   UIKitCore           0x00007fff23456abc -[UIViewController loadViewIfRequired] + 200
5   UIKitCore           0x00007fff23456def -[UIViewController view] + 50
6   MyApp               0x000000010a234999 MyApp.AppDelegate.application(_:didFinishLaunchingWithOptions:) -> Bool + 456
7   UIKitCore           0x00007fff23457000 -[UIApplication _handleDelegateCallbacksWithOptions:isSuspended:restoreState:] + 300"#;

        let result = compress_swift_crash(input);

        // Must keep the crash header
        assert!(
            result.contains("Thread 0 Crashed:"),
            "Should keep crash header: {}",
            result
        );
        // Must keep app frames
        assert!(
            result.contains("MyApp"),
            "Should keep app frames: {}",
            result
        );
        assert!(
            result.contains("viewDidLoad"),
            "Should keep app symbols: {}",
            result
        );
        assert!(
            result.contains("didFinishLaunchingWithOptions"),
            "Should keep app delegate frame: {}",
            result
        );
        // Must collapse framework frames
        assert!(
            !result.contains("libswiftCore.dylib"),
            "Should hide libswiftCore frames: {}",
            result
        );
        assert!(
            !result.contains("UIKitCore"),
            "Should hide UIKitCore frames: {}",
            result
        );
        // Must show hidden count
        assert!(
            result.contains("framework frames hidden"),
            "Should show hidden frame count: {}",
            result
        );
    }

    #[test]
    fn test_compress_xcode_log() {
        let input = r#"CompileSwift normal arm64 /path/to/file1.swift
CompileSwift normal arm64 /path/to/file2.swift
CompileSwift normal arm64 /path/to/file3.swift
CompileSwift normal arm64 /path/to/file4.swift
CompileSwift normal arm64 /path/to/file5.swift
Ld /build/Debug/MyApp normal arm64
** BUILD SUCCEEDED **"#;

        let result = compress_xcode_log(input);
        assert!(
            result.contains("CompileSwift: 5 files compiled"),
            "Should collapse CompileSwift lines: {}",
            result
        );
        assert!(
            !result.contains("/path/to/file1.swift"),
            "Should strip individual file paths: {}",
            result
        );
        assert!(
            result.contains("BUILD SUCCEEDED"),
            "Should keep build result: {}",
            result
        );
    }

    #[test]
    fn test_filter_passthrough_no_patterns() {
        let input = "Hello, world!\nThis is plain output.";
        let result = filter_swift_build(input);
        // When no patterns match, should return original
        assert_eq!(result, input);
    }

    #[test]
    fn test_filter_build_steps_stripped() {
        let input = r#"ProcessInfoPlistFile /build/Info.plist
CopySwiftLibs /build/Debug/MyApp.app
CodeSign /build/Debug/MyApp.app
CompileSwift normal arm64 /path/to/file.swift
** BUILD SUCCEEDED **"#;

        let result = filter_swift_build(input);
        assert!(
            !result.contains("ProcessInfoPlistFile"),
            "Should strip ProcessInfoPlistFile: {}",
            result
        );
        assert!(
            !result.contains("CopySwiftLibs"),
            "Should strip CopySwiftLibs: {}",
            result
        );
        assert!(
            !result.contains("CodeSign"),
            "Should strip CodeSign: {}",
            result
        );
        assert!(
            result.contains("1 Swift file compiled"),
            "Should count compile: {}",
            result
        );
    }
}
