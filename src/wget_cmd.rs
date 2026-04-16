use crate::tracking;
use crate::utils::resolved_command;
use anyhow::{Context, Result};

/// Compact wget - strips progress bars, shows only result
pub fn run(url: &str, args: &[String], verbose: u8) -> Result<()> {
    let timer = tracking::TimedExecution::start();

    if verbose > 0 {
        eprintln!("wget: {}", url);
    }

    // Run wget normally but capture output to parse it
    let mut cmd_args: Vec<&str> = vec![];

    // Add user args
    for arg in args {
        cmd_args.push(arg);
    }
    cmd_args.push(url);

    let output = resolved_command("wget")
        .args(&cmd_args)
        .output()
        .context("Failed to run wget")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let raw_output = format!("{}\n{}", stderr, stdout);

    if output.status.success() {
        let filename = extract_filename_from_output(&stderr, url, args);
        let size = get_file_size(&filename);
        let msg = format!(
            "{} ok | {} | {}",
            compact_url(url),
            filename,
            format_size(size)
        );
        println!("{}", msg);
        timer.track(
            &format!("wget {}", url),
            "contextzip wget",
            &raw_output,
            &msg,
        );
    } else {
        let error = parse_error(&stderr, &stdout);
        let msg = format!("{} FAILED: {}", compact_url(url), error);
        println!("{}", msg);
        timer.track(
            &format!("wget {}", url),
            "contextzip wget",
            &raw_output,
            &msg,
        );
        std::process::exit(output.status.code().unwrap_or(1));
    }

    Ok(())
}

/// Run wget and output to stdout (for piping)
pub fn run_stdout(url: &str, args: &[String], verbose: u8) -> Result<()> {
    let timer = tracking::TimedExecution::start();

    if verbose > 0 {
        eprintln!("wget: {} -> stdout", url);
    }

    let mut cmd_args = vec!["-q", "-O", "-"];
    for arg in args {
        cmd_args.push(arg);
    }
    cmd_args.push(url);

    let output = resolved_command("wget")
        .args(&cmd_args)
        .output()
        .context("Failed to run wget")?;

    if output.status.success() {
        let content = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let raw_output = content.to_string();

        let mut compressed = String::new();
        if total > 20 {
            compressed.push_str(&format!(
                "{} ok | {} lines | {}\n",
                compact_url(url),
                total,
                format_size(output.stdout.len() as u64)
            ));
            compressed.push_str("--- first 10 lines ---\n");
            for line in lines.iter().take(10) {
                compressed.push_str(&format!("{}\n", truncate_line(line, 100)));
            }
            compressed.push_str(&format!("... +{} more lines", total - 10));
        } else {
            compressed.push_str(&format!("{} ok | {} lines\n", compact_url(url), total));
            for line in &lines {
                compressed.push_str(&format!("{}\n", line));
            }
        }
        print!("{}", compressed);
        timer.track(
            &format!("wget -O - {}", url),
            "contextzip wget -o",
            &raw_output,
            &compressed,
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error = parse_error(&stderr, "");
        let msg = format!("{} FAILED: {}", compact_url(url), error);
        println!("{}", msg);
        timer.track(
            &format!("wget -O - {}", url),
            "contextzip wget -o",
            &stderr,
            &msg,
        );
        std::process::exit(output.status.code().unwrap_or(1));
    }

    Ok(())
}

fn extract_filename_from_output(stderr: &str, url: &str, args: &[String]) -> String {
    // Check for -O argument first
    for (i, arg) in args.iter().enumerate() {
        if arg == "-O" || arg == "--output-document" {
            if let Some(name) = args.get(i + 1) {
                return name.clone();
            }
        }
        if let Some(name) = arg.strip_prefix("-O") {
            return name.to_string();
        }
    }

    // Parse wget output for "Sauvegarde en" or "Saving to"
    for line in stderr.lines() {
        // French: Sauvegarde en : « filename »
        if line.contains("Sauvegarde en") || line.contains("Saving to") {
            // Use char-based parsing to handle Unicode properly
            let chars: Vec<char> = line.chars().collect();
            let mut start_idx = None;
            let mut end_idx = None;

            for (i, c) in chars.iter().enumerate() {
                if *c == '«' || (*c == '\'' && start_idx.is_none()) {
                    start_idx = Some(i);
                }
                if *c == '»' || (*c == '\'' && start_idx.is_some()) {
                    end_idx = Some(i);
                }
            }

            if let (Some(s), Some(e)) = (start_idx, end_idx) {
                if e > s + 1 {
                    let filename: String = chars[s + 1..e].iter().collect();
                    return filename.trim().to_string();
                }
            }
        }
    }

    // Fallback: extract from URL
    let path = url.rsplit("://").next().unwrap_or(url);
    let filename = path
        .rsplit('/')
        .next()
        .unwrap_or("index.html")
        .split('?')
        .next()
        .unwrap_or("index.html");

    if filename.is_empty() || !filename.contains('.') {
        "index.html".to_string()
    } else {
        filename.to_string()
    }
}

fn get_file_size(filename: &str) -> u64 {
    std::fs::metadata(filename).map(|m| m.len()).unwrap_or(0)
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "?".to_string();
    }
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn compact_url(url: &str) -> String {
    // Remove protocol
    let without_proto = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Truncate if too long
    let chars: Vec<char> = without_proto.chars().collect();
    if chars.len() <= 50 {
        without_proto.to_string()
    } else {
        let prefix: String = chars[..25].iter().collect();
        let suffix: String = chars[chars.len() - 20..].iter().collect();
        format!("{}...{}", prefix, suffix)
    }
}

#[allow(dead_code)]
fn parse_error(stderr: &str, stdout: &str) -> String {
    // Common wget error patterns
    let combined = format!("{}\n{}", stderr, stdout);

    if combined.contains("404") {
        return "404 Not Found".to_string();
    }
    if combined.contains("403") {
        return "403 Forbidden".to_string();
    }
    if combined.contains("401") {
        return "401 Unauthorized".to_string();
    }
    if combined.contains("500") {
        return "500 Server Error".to_string();
    }
    if combined.contains("Connection refused") {
        return "Connection refused".to_string();
    }
    if combined.contains("unable to resolve") || combined.contains("Name or service not known") {
        return "DNS lookup failed".to_string();
    }
    if combined.contains("timed out") {
        return "Connection timed out".to_string();
    }
    if combined.contains("SSL") || combined.contains("certificate") {
        return "SSL/TLS error".to_string();
    }

    // Return first meaningful line
    for line in stderr.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("--") {
            if trimmed.len() > 60 {
                let t: String = trimmed.chars().take(60).collect();
                return format!("{}...", t);
            }
            return trimmed.to_string();
        }
    }

    "Unknown error".to_string()
}

fn truncate_line(line: &str, max: usize) -> String {
    if line.len() <= max {
        line.to_string()
    } else {
        let t: String = line.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_url_strips_protocol() {
        assert_eq!(compact_url("https://example.com/path"), "example.com/path");
        assert_eq!(compact_url("http://localhost:8080"), "localhost:8080");
    }

    #[test]
    fn compact_url_keeps_short_urls_intact() {
        let short = "example.com/short";
        assert_eq!(compact_url(&format!("https://{}", short)), short);
    }

    #[test]
    fn compact_url_truncates_long_urls_with_ellipsis() {
        let long = format!(
            "https://{}/{}",
            "a".repeat(40),
            "b".repeat(40)
        );
        let out = compact_url(&long);
        assert!(out.contains("..."), "expected ellipsis in {}", out);
        assert!(out.len() < long.len());
    }

    #[test]
    fn format_size_uses_human_readable_units() {
        assert_eq!(format_size(0), "?");
        assert_eq!(format_size(100), "100B");
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1536), "1.5KB");
        assert_eq!(format_size(1024 * 1024), "1.0MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0GB");
    }

    #[test]
    fn parse_error_recognizes_http_status_codes() {
        assert_eq!(parse_error("ERROR 404: Not Found", ""), "404 Not Found");
        assert_eq!(parse_error("ERROR 403", ""), "403 Forbidden");
        assert_eq!(parse_error("ERROR 401", ""), "401 Unauthorized");
        assert_eq!(parse_error("ERROR 500", ""), "500 Server Error");
    }

    #[test]
    fn parse_error_recognizes_network_failures() {
        assert_eq!(
            parse_error("Connection refused on port 80", ""),
            "Connection refused"
        );
        assert_eq!(
            parse_error("wget: unable to resolve host", ""),
            "DNS lookup failed"
        );
        assert_eq!(parse_error("Connection timed out", ""), "Connection timed out");
    }

    #[test]
    fn parse_error_recognizes_ssl_problems() {
        assert_eq!(
            parse_error("SSL handshake failed", ""),
            "SSL/TLS error"
        );
        assert_eq!(
            parse_error("certificate verification failed", ""),
            "SSL/TLS error"
        );
    }

    #[test]
    fn parse_error_falls_back_to_first_meaningful_line() {
        let err = parse_error("first error line\n--debug header", "");
        assert_eq!(err, "first error line");
    }

    #[test]
    fn parse_error_returns_unknown_for_empty_input() {
        assert_eq!(parse_error("", ""), "Unknown error");
        assert_eq!(parse_error("--just headers--", ""), "Unknown error");
    }

    #[test]
    fn truncate_line_short_lines_unchanged() {
        assert_eq!(truncate_line("short", 100), "short");
    }

    #[test]
    fn truncate_line_long_lines_get_ellipsis() {
        let line = "a".repeat(200);
        let out = truncate_line(&line, 50);
        assert!(out.ends_with("..."));
        assert_eq!(out.chars().count(), 50);
    }

    #[test]
    fn extract_filename_prefers_explicit_O_arg() {
        let args = vec!["-O".to_string(), "myfile.tar.gz".to_string()];
        assert_eq!(
            extract_filename_from_output("", "https://example.com/x", &args),
            "myfile.tar.gz"
        );
    }

    #[test]
    fn extract_filename_falls_back_to_url_basename() {
        assert_eq!(
            extract_filename_from_output("", "https://example.com/path/file.tar.gz", &[]),
            "file.tar.gz"
        );
        assert_eq!(
            extract_filename_from_output("", "https://example.com/path/archive.zip?token=x", &[]),
            "archive.zip"
        );
    }

    #[test]
    fn extract_filename_defaults_when_no_basename() {
        assert_eq!(
            extract_filename_from_output("", "https://example.com/", &[]),
            "index.html"
        );
        assert_eq!(
            extract_filename_from_output("", "https://example.com/folder", &[]),
            "index.html"
        );
    }

    #[test]
    fn extract_filename_parses_french_wget_output() {
        let stderr = "Sauvegarde en : « downloaded.txt »";
        assert_eq!(
            extract_filename_from_output(stderr, "https://example.com/x", &[]),
            "downloaded.txt"
        );
    }

    #[test]
    fn get_file_size_returns_zero_for_missing_file() {
        // Defensive contract: don't panic on missing file, return 0
        assert_eq!(get_file_size("/nonexistent/path/to/file.xyz"), 0);
    }
}
