use crate::tracking;
use crate::utils::{resolved_command, truncate};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::ffi::OsString;

lazy_static! {
    // Kotlin compiler error/warning lines
    static ref KOTLIN_ERROR_RE: Regex = Regex::new(r"^[ew]:\s+").unwrap();
    // Gradle task lines: "> Task :app:compileDebugKotlin"
    static ref GRADLE_TASK_RE: Regex = Regex::new(r"^>\s+Task\s+:").unwrap();
    // Gradle BUILD result line
    static ref GRADLE_BUILD_RESULT_RE: Regex = Regex::new(r"^BUILD\s+(SUCCESSFUL|FAILED)").unwrap();
    // Gradle actionable tasks summary (e.g. "42 actionable tasks: 40 executed, 2 up-to-date")
    static ref GRADLE_ACTIONABLE_RE: Regex = Regex::new(r"^\d+\s+actionable\s+task").unwrap();
    // JVM/Android stacktrace frame (matched against trimmed line)
    static ref JVM_FRAME_RE: Regex = Regex::new(r"^at\s+([\w.$]+)\(([\w.]+:\d+)\)").unwrap();
    // "Caused by:" lines
    static ref CAUSED_BY_RE: Regex = Regex::new(r"^\s*Caused by:\s+").unwrap();
    // "... N more" lines
    static ref MORE_FRAMES_RE: Regex = Regex::new(r"^\s+\.\.\.\s+\d+\s+more").unwrap();
    // Framework package prefixes to collapse (matched against trimmed line)
    static ref FRAMEWORK_RE: Regex = Regex::new(
        r"^at\s+(android\.|androidx\.|java\.|javax\.|kotlin\.|kotlinx\.|dalvik\.|com\.android\.|org\.jetbrains\.|sun\.|jdk\.internal\.|org\.gradle\.)"
    ).unwrap();
    // Gradle download/resolution noise
    static ref GRADLE_DOWNLOAD_RE: Regex = Regex::new(r"^(Downloading|Download)\s+https?://").unwrap();
    // Gradle configuration/dependency lines
    static ref GRADLE_CONFIG_RE: Regex = Regex::new(r"^(> Configure |> Transform |> Resolve )").unwrap();
    // Kotlin compilation progress percentage
    static ref KOTLIN_PROGRESS_RE: Regex = Regex::new(r"^\[[\d:]+\]\s").unwrap();
    // Exception class line (e.g. "java.lang.NullPointerException: message")
    static ref EXCEPTION_RE: Regex = Regex::new(r"^([\w.$]+Exception|[\w.$]+Error):?\s").unwrap();
}

pub fn run_build(args: &[String], verbose: u8) -> Result<()> {
    run_gradle_filtered("build", args, verbose, filter_kotlin_build)
}

pub fn run_test(args: &[String], verbose: u8) -> Result<()> {
    run_gradle_filtered("test", args, verbose, filter_kotlin_build)
}

pub fn run_lint(args: &[String], verbose: u8) -> Result<()> {
    run_gradle_filtered("lint", args, verbose, filter_kotlin_build)
}

pub fn run_other(args: &[OsString], verbose: u8) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("gradle: no subcommand specified");
    }

    let timer = tracking::TimedExecution::start();

    let subcommand = args[0].to_string_lossy();
    let mut cmd = resolved_command("gradle");
    cmd.arg(&*subcommand);

    for arg in &args[1..] {
        cmd.arg(arg);
    }

    if verbose > 0 {
        eprintln!("Running: gradle {} ...", subcommand);
    }

    let output = cmd
        .output()
        .with_context(|| format!("Failed to run gradle {}", subcommand))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = format!("{}\n{}", stdout, stderr);

    print!("{}", stdout);
    eprint!("{}", stderr);

    timer.track(
        &format!("gradle {}", subcommand),
        &format!("contextzip gradle {}", subcommand),
        &raw,
        &raw,
    );

    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }

    Ok(())
}

/// Generic gradle command runner with filtering
fn run_gradle_filtered<F>(subcommand: &str, args: &[String], verbose: u8, filter_fn: F) -> Result<()>
where
    F: Fn(&str) -> String,
{
    let timer = tracking::TimedExecution::start();

    let mut cmd = resolved_command("gradle");
    cmd.arg(subcommand);

    for arg in args {
        cmd.arg(arg);
    }

    if verbose > 0 {
        eprintln!("Running: gradle {} {}", subcommand, args.join(" "));
    }

    let output = cmd
        .output()
        .with_context(|| format!("Failed to run gradle {}. Is Gradle installed?", subcommand))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = format!("{}\n{}", stdout, stderr);

    let exit_code = output
        .status
        .code()
        .unwrap_or(if output.status.success() { 0 } else { 1 });
    let filtered = filter_fn(&raw);

    if let Some(hint) = crate::tee::tee_and_hint(&raw, &format!("gradle_{}", subcommand), exit_code)
    {
        if !filtered.is_empty() {
            println!("{}\n{}", filtered, hint);
        } else {
            println!("{}", hint);
        }
    } else if !filtered.is_empty() {
        println!("{}", filtered);
    }

    timer.track(
        &format!("gradle {} {}", subcommand, args.join(" ")),
        &format!("contextzip gradle {} {}", subcommand, args.join(" ")),
        &raw,
        &filtered,
    );

    if !output.status.success() {
        std::process::exit(exit_code);
    }

    Ok(())
}

/// Filter Kotlin/Gradle build output.
///
/// Keeps:
///   - Kotlin compiler errors and warnings (e: / w: lines)
///   - Exception/error class lines and app-code stack frames
///   - BUILD SUCCESSFUL/FAILED result lines
///   - Caused-by chains
///
/// Collapses:
///   - Gradle task lines (> Task :app:...) into a count
///   - Framework stack frames (android.*, kotlin.*, java.*, etc.)
///   - Gradle download/configuration noise
pub fn filter_kotlin_build(output: &str) -> String {
    let mut result: Vec<String> = Vec::new();
    let mut task_count: usize = 0;
    let mut framework_hidden: usize = 0;
    let mut kotlin_errors: Vec<String> = Vec::new();
    let mut kotlin_warnings: Vec<String> = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Kotlin compiler errors/warnings — collect separately
        if KOTLIN_ERROR_RE.is_match(trimmed) {
            if trimmed.starts_with("e:") {
                kotlin_errors.push(trimmed.to_string());
            } else {
                kotlin_warnings.push(trimmed.to_string());
            }
            continue;
        }

        // Gradle task lines — just count them
        if GRADLE_TASK_RE.is_match(trimmed) {
            task_count += 1;
            continue;
        }

        // Gradle download/config noise — skip
        if GRADLE_DOWNLOAD_RE.is_match(trimmed) || GRADLE_CONFIG_RE.is_match(trimmed) {
            continue;
        }

        // Kotlin compilation progress — skip
        if KOTLIN_PROGRESS_RE.is_match(trimmed) {
            continue;
        }

        // BUILD result line — flush and keep
        if GRADLE_BUILD_RESULT_RE.is_match(trimmed) {
            flush_framework_hidden(&mut result, &mut framework_hidden);
            flush_tasks(&mut result, &mut task_count);
            result.push(trimmed.to_string());
            continue;
        }

        // Actionable tasks summary — keep
        if GRADLE_ACTIONABLE_RE.is_match(trimmed) {
            result.push(trimmed.to_string());
            continue;
        }

        // Exception/error header lines — keep
        if EXCEPTION_RE.is_match(trimmed) {
            flush_framework_hidden(&mut result, &mut framework_hidden);
            result.push(trimmed.to_string());
            continue;
        }

        // Caused by — keep
        if CAUSED_BY_RE.is_match(trimmed) {
            flush_framework_hidden(&mut result, &mut framework_hidden);
            result.push(line.to_string());
            continue;
        }

        // "... N more" — keep
        if MORE_FRAMES_RE.is_match(trimmed) {
            flush_framework_hidden(&mut result, &mut framework_hidden);
            result.push(line.to_string());
            continue;
        }

        // JVM stack frame — classify as framework or app
        if JVM_FRAME_RE.is_match(trimmed) {
            if FRAMEWORK_RE.is_match(trimmed) {
                framework_hidden += 1;
            } else {
                flush_framework_hidden(&mut result, &mut framework_hidden);
                result.push(line.to_string());
            }
            continue;
        }

        // Everything else — keep (error messages, build output, etc.)
        flush_framework_hidden(&mut result, &mut framework_hidden);
        result.push(line.to_string());
    }

    flush_framework_hidden(&mut result, &mut framework_hidden);
    flush_tasks(&mut result, &mut task_count);

    // Build final output
    let mut final_output = String::new();

    // Errors first
    if !kotlin_errors.is_empty() {
        final_output.push_str(&format!(
            "Kotlin: {} error{}\n",
            kotlin_errors.len(),
            if kotlin_errors.len() == 1 { "" } else { "s" }
        ));
        for (i, err) in kotlin_errors.iter().take(20).enumerate() {
            final_output.push_str(&format!("{}. {}\n", i + 1, truncate(err, 150)));
        }
        if kotlin_errors.len() > 20 {
            final_output.push_str(&format!("... +{} more errors\n", kotlin_errors.len() - 20));
        }
    }

    // Warnings (limited)
    if !kotlin_warnings.is_empty() {
        final_output.push_str(&format!(
            "Kotlin: {} warning{}\n",
            kotlin_warnings.len(),
            if kotlin_warnings.len() == 1 { "" } else { "s" }
        ));
        for warn in kotlin_warnings.iter().take(5) {
            final_output.push_str(&format!("  {}\n", truncate(warn, 150)));
        }
        if kotlin_warnings.len() > 5 {
            final_output.push_str(&format!(
                "  ... +{} more warnings\n",
                kotlin_warnings.len() - 5
            ));
        }
    }

    // Rest of the filtered output
    let rest = result.join("\n");
    let rest = rest.trim();
    if !rest.is_empty() {
        if !final_output.is_empty() {
            final_output.push('\n');
        }
        final_output.push_str(rest);
    }

    if final_output.is_empty() {
        return "Gradle: Success".to_string();
    }

    final_output.trim().to_string()
}

fn flush_framework_hidden(result: &mut Vec<String>, count: &mut usize) {
    if *count > 0 {
        result.push(format!("\t(+ {} framework frames hidden)", *count));
        *count = 0;
    }
}

fn flush_tasks(result: &mut Vec<String>, count: &mut usize) {
    if *count > 0 {
        result.push(format!("({} Gradle tasks executed)", *count));
        *count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kotlin_compiler_errors() {
        let input = r#"> Task :app:compileDebugKotlin
e: file:///path/to/App.kt:15:42 Unresolved reference: foo
e: file:///path/to/Bar.kt:30:10 Type mismatch: inferred type is String but Int was expected
w: file:///path/to/Utils.kt:30:1 Parameter 'x' is never used
> Task :app:compileDebugJavaWithJavac
> Task :app:mergeDebugResources
BUILD FAILED in 12s
3 actionable tasks: 2 executed, 1 up-to-date"#;

        let result = filter_kotlin_build(input);

        // Must report error count
        assert!(result.contains("Kotlin: 2 errors"), "result: {}", result);
        // Must keep error details
        assert!(result.contains("Unresolved reference: foo"));
        assert!(result.contains("Type mismatch"));
        // Must keep warning count
        assert!(result.contains("Kotlin: 1 warning"));
        assert!(result.contains("Parameter 'x' is never used"));
        // Must collapse task lines
        assert!(!result.contains("> Task :app:compileDebugKotlin"));
        assert!(!result.contains("> Task :app:compileDebugJavaWithJavac"));
        assert!(result.contains("Gradle tasks executed"));
        // Must keep BUILD result
        assert!(result.contains("BUILD FAILED"));
    }

    #[test]
    fn test_android_stacktrace_compression() {
        let input = r#"java.lang.NullPointerException: Attempt to invoke virtual method on null
    at com.myapp.ui.MainActivity.onCreate(MainActivity.kt:42)
    at com.myapp.data.UserRepository.fetch(UserRepository.kt:18)
    at android.app.Activity.performCreate(Activity.java:8051)
    at android.app.Instrumentation.callActivityOnCreate(Instrumentation.java:1329)
    at android.app.ActivityThread.handleLaunchActivity(ActivityThread.java:3460)
    at androidx.fragment.app.FragmentActivity.onCreate(FragmentActivity.java:321)
    at kotlin.coroutines.jvm.internal.BaseContinuationImpl.resumeWith(ContinuationImpl.kt:33)
    at kotlinx.coroutines.DispatchedTask.run(DispatchedTask.kt:106)
    at java.lang.reflect.Method.invoke(Method.java:372)
    at dalvik.system.NativeStart.main(NativeStart.java:52)"#;

        let result = filter_kotlin_build(input);

        // Must keep exception header
        assert!(result.contains("NullPointerException"));
        // Must keep app frames
        assert!(result.contains("com.myapp.ui.MainActivity.onCreate"));
        assert!(result.contains("com.myapp.data.UserRepository.fetch"));
        // Must hide framework frames
        assert!(!result.contains("android.app.Activity.performCreate"));
        assert!(!result.contains("android.app.Instrumentation"));
        assert!(!result.contains("androidx.fragment"));
        assert!(!result.contains("kotlin.coroutines.jvm.internal"));
        assert!(!result.contains("kotlinx.coroutines"));
        assert!(!result.contains("java.lang.reflect.Method"));
        assert!(!result.contains("dalvik.system"));
        // Must show hidden count
        assert!(result.contains("framework frames hidden"));
    }

    #[test]
    fn test_gradle_task_collapse() {
        let input = r#"> Task :app:preBuild UP-TO-DATE
> Task :app:preDebugBuild UP-TO-DATE
> Task :app:compileDebugAidl NO-SOURCE
> Task :app:compileDebugRenderscript NO-SOURCE
> Task :app:generateDebugBuildConfig
> Task :app:checkDebugAarMetadata
> Task :app:generateDebugResValues
> Task :app:generateDebugResources
> Task :app:mergeDebugResources
> Task :app:createDebugCompatibleScreenManifests
> Task :app:extractDeepLinksDebug
> Task :app:processDebugMainManifest
> Task :app:processDebugManifest
> Task :app:processDebugManifestForPackage
> Task :app:processDebugResources
> Task :app:compileDebugKotlin
> Task :app:javaPreCompileDebug
> Task :app:compileDebugJavaWithJavac
> Task :app:mergeDebugShaders
> Task :app:compileDebugShaders NO-SOURCE
> Task :app:generateDebugAssets UP-TO-DATE
> Task :app:mergeDebugAssets
> Task :app:compressDebugAssets
> Task :app:processDebugJavaRes NO-SOURCE
> Task :app:mergeDebugJavaResource
> Task :app:checkDebugDuplicateClasses
> Task :app:desugarDebugFileDependencies
> Task :app:mergeExtDexDebug
> Task :app:mergeDexDebug
> Task :app:validateSigningDebug
> Task :app:packageDebug
> Task :app:assembleDebug
BUILD SUCCESSFUL in 45s
32 actionable tasks: 30 executed, 2 up-to-date"#;

        let result = filter_kotlin_build(input);

        // Must NOT contain individual task lines
        assert!(!result.contains("> Task :app:"));
        // Must show task count summary
        assert!(result.contains("32 Gradle tasks executed"));
        // Must keep BUILD SUCCESSFUL
        assert!(result.contains("BUILD SUCCESSFUL"));
        // Must keep actionable tasks summary
        assert!(result.contains("actionable tasks"));
    }

    #[test]
    fn test_empty_successful_build() {
        let input = "";
        let result = filter_kotlin_build(input);
        assert_eq!(result, "Gradle: Success");
    }

    #[test]
    fn test_gradle_download_noise_removed() {
        let input = r#"Downloading https://services.gradle.org/distributions/gradle-8.0-bin.zip
> Configure project :app
> Task :app:compileDebugKotlin
BUILD SUCCESSFUL in 15s"#;

        let result = filter_kotlin_build(input);

        assert!(!result.contains("Downloading https://"));
        assert!(!result.contains("> Configure project"));
        assert!(result.contains("BUILD SUCCESSFUL"));
    }

    #[test]
    fn test_caused_by_chain_preserved() {
        let input = r#"java.lang.RuntimeException: Unable to start activity
    at android.app.ActivityThread.performLaunchActivity(ActivityThread.java:3449)
    at android.app.ActivityThread.handleLaunchActivity(ActivityThread.java:3601)
Caused by: kotlin.UninitializedPropertyAccessException: lateinit property viewModel has not been initialized
    at com.myapp.ui.MainActivity.onCreate(MainActivity.kt:25)
    at android.app.Activity.performCreate(Activity.java:8051)"#;

        let result = filter_kotlin_build(input);

        // Must keep Caused by chain
        assert!(result.contains("Caused by: kotlin.UninitializedPropertyAccessException"));
        // Must keep app frame
        assert!(result.contains("com.myapp.ui.MainActivity.onCreate"));
        // Must hide android framework frames
        assert!(!result.contains("android.app.ActivityThread.performLaunchActivity"));
        assert!(!result.contains("android.app.Activity.performCreate"));
    }

    #[test]
    fn test_mixed_errors_and_stacktrace() {
        let input = r#"e: file:///src/App.kt:10:5 Unresolved reference: bar
> Task :app:compileDebugKotlin FAILED
java.lang.NullPointerException: null
    at com.myapp.Processor.run(Processor.kt:42)
    at java.lang.Thread.run(Thread.java:750)
BUILD FAILED in 5s"#;

        let result = filter_kotlin_build(input);

        // Errors listed first
        assert!(result.contains("Kotlin: 1 error"));
        assert!(result.contains("Unresolved reference: bar"));
        // Stacktrace compressed
        assert!(result.contains("com.myapp.Processor.run"));
        assert!(!result.contains("java.lang.Thread.run"));
        // BUILD result kept
        assert!(result.contains("BUILD FAILED"));
    }
}
