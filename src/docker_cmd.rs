use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // ---> abc123def456
    static ref STEP_HASH_RE: Regex = Regex::new(r"^--->+ [0-9a-f]{6,}").unwrap();
    // ---> Using cache
    static ref USING_CACHE_RE: Regex = Regex::new(r"^--->+ Using cache").unwrap();
    // Step N/M : <instruction>
    static ref STEP_RE: Regex = Regex::new(r"^Step (\d+)/(\d+) : (.+)").unwrap();
    // Downloading [=====>    ] 45%
    static ref PULL_PROGRESS_RE: Regex =
        Regex::new(r"(?:Downloading|Extracting|Pulling)\s*\[").unwrap();
    // Removing intermediate container abc123
    static ref REMOVING_CONTAINER_RE: Regex =
        Regex::new(r"^Removing intermediate container [0-9a-f]+").unwrap();
    // Successfully built <hash>
    static ref SUCCESS_BUILT_RE: Regex =
        Regex::new(r"^Successfully built [0-9a-f]+").unwrap();
    // Successfully tagged <tag>
    static ref SUCCESS_TAGGED_RE: Regex =
        Regex::new(r"^Successfully tagged (.+)").unwrap();
    // Exit code pattern in error output
    static ref EXIT_CODE_RE: Regex =
        Regex::new(r"(?i)exit (?:code|status)[:\s]+(\d+)").unwrap();
    // Non-zero return code from step
    static ref RETURN_CODE_RE: Regex =
        Regex::new(r"returned a non-zero code:\s*(\d+)").unwrap();
}

/// Compresses Docker build log output.
///
/// On success: produces a 1-line summary with tag and step/cache counts.
/// On failure: preserves the failed step, 2 prior steps for context,
/// the full error message, and exit code.
/// Non-docker output passes through unchanged.
#[allow(dead_code)]
pub fn compress_docker_log(input: &str) -> String {
    if input.trim().is_empty() {
        return input.to_string();
    }

    // Detect if this is docker build output by looking for Step lines
    let lines: Vec<&str> = input.lines().collect();
    let has_steps = lines.iter().any(|l| STEP_RE.is_match(l.trim()));

    if !has_steps {
        // Not docker build output — passthrough
        return input.to_string();
    }

    // Parse all steps
    let mut steps: Vec<StepInfo> = Vec::new();
    let mut current_step: Option<StepInfo> = None;

    for line in &lines {
        let trimmed = line.trim();

        if let Some(caps) = STEP_RE.captures(trimmed) {
            // Save previous step
            if let Some(step) = current_step.take() {
                steps.push(step);
            }
            let step_num: usize = caps[1].parse().unwrap_or(0);
            let total: usize = caps[2].parse().unwrap_or(0);
            let instruction = caps[3].to_string();
            current_step = Some(StepInfo {
                step_num,
                total,
                instruction,
                cached: false,
                failed: false,
                error_lines: Vec::new(),
            });
        } else if let Some(ref mut step) = current_step {
            if USING_CACHE_RE.is_match(trimmed) {
                step.cached = true;
            } else if STEP_HASH_RE.is_match(trimmed)
                || REMOVING_CONTAINER_RE.is_match(trimmed)
                || PULL_PROGRESS_RE.is_match(trimmed)
            {
                // Noise lines — skip
            } else if !trimmed.is_empty()
                && !SUCCESS_BUILT_RE.is_match(trimmed)
                && !SUCCESS_TAGGED_RE.is_match(trimmed)
            {
                // Could be command output or error
                step.error_lines.push(trimmed.to_string());
            }
        }
    }
    // Push last step
    if let Some(step) = current_step {
        steps.push(step);
    }

    // Detect success vs failure
    let is_success = lines.iter().any(|l| {
        let t = l.trim();
        SUCCESS_BUILT_RE.is_match(t) || SUCCESS_TAGGED_RE.is_match(t)
    });

    // Also check for explicit failure signals
    let has_failure = lines.iter().any(|l| {
        let t = l.trim();
        RETURN_CODE_RE.is_match(t)
            || (t.contains("The command") && t.contains("returned a non-zero code"))
    });

    let is_success = is_success && !has_failure;

    if is_success {
        format_success(&steps, &lines)
    } else {
        format_failure(&steps, &lines)
    }
}

#[allow(dead_code)]
struct StepInfo {
    step_num: usize,
    total: usize,
    instruction: String,
    cached: bool,
    failed: bool,
    error_lines: Vec<String>,
}

fn format_success(steps: &[StepInfo], lines: &[&str]) -> String {
    let total_steps = steps.last().map(|s| s.total).unwrap_or(steps.len());
    let cached_count = steps.iter().filter(|s| s.cached).count();

    // Try to extract the tag from "Successfully tagged <tag>"
    let tag = lines
        .iter()
        .rev()
        .find_map(|l| SUCCESS_TAGGED_RE.captures(l.trim()))
        .map(|caps| caps[1].to_string())
        .unwrap_or_else(|| "image".to_string());

    format!(
        "\u{2713} built {} ({} steps, {} cached)",
        tag, total_steps, cached_count
    )
}

fn format_failure(steps: &[StepInfo], lines: &[&str]) -> String {
    // Find the failed step — the last step that has error lines or
    // the step referenced in the error message
    let mut failed_idx: Option<usize> = None;

    // Check for non-zero return code to identify failed step
    for line in lines {
        if RETURN_CODE_RE.is_match(line.trim()) {
            // The failed step is typically the last one
            if !steps.is_empty() {
                failed_idx = Some(steps.len() - 1);
            }
            break;
        }
    }

    // If no explicit failure marker, assume last step failed
    if failed_idx.is_none() && !steps.is_empty() {
        failed_idx = Some(steps.len() - 1);
    }

    let Some(fi) = failed_idx else {
        return "✗ Docker build failed".to_string();
    };

    let failed_step = &steps[fi];
    let total = failed_step.total;

    let mut result = format!(
        "\u{2717} Docker build failed at step {}/{}\n",
        failed_step.step_num, total
    );

    // Show 2 steps before failed step for context
    let context_start = fi.saturating_sub(2);
    for s in steps.iter().take(fi).skip(context_start) {
        let cache_marker = if s.cached { "(cached \u{2713})" } else { "" };
        result.push_str(&format!(
            "\nStep {}/{} : {}    {}\n",
            s.step_num, s.total, s.instruction, cache_marker
        ));
    }

    // Show failed step
    result.push_str(&format!(
        "Step {}/{} : {}        \u{2190} FAILED\n",
        failed_step.step_num, failed_step.total, failed_step.instruction
    ));

    // Show error lines from the failed step
    for err_line in &failed_step.error_lines {
        result.push_str(&format!("  {}\n", err_line));
    }

    // Extract and show exit code
    let exit_code = extract_exit_code(lines);
    if let Some(code) = exit_code {
        result.push_str(&format!("  Exit code: {}\n", code));
    }

    result.trim_end().to_string()
}

fn extract_exit_code(lines: &[&str]) -> Option<u32> {
    for line in lines {
        let trimmed = line.trim();
        if let Some(caps) = EXIT_CODE_RE.captures(trimmed) {
            if let Ok(code) = caps[1].parse::<u32>() {
                return Some(code);
            }
        }
        if let Some(caps) = RETURN_CODE_RE.captures(trimmed) {
            if let Ok(code) = caps[1].parse::<u32>() {
                return Some(code);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_successful_build_one_line_summary() {
        let input = "\
Step 1/12 : FROM node:20-alpine
 ---> abc123def456
Step 2/12 : WORKDIR /app
 ---> Using cache
 ---> def456789012
Step 3/12 : COPY package*.json ./
 ---> Using cache
 ---> 111222333444
Step 4/12 : RUN npm install
 ---> Using cache
 ---> 555666777888
Step 5/12 : COPY . .
 ---> abc111222333
Step 6/12 : RUN npm run build
 ---> Running in abc999888777
Step 7/12 : FROM nginx:alpine
 ---> fff000111222
Step 8/12 : COPY --from=0 /app/build /usr/share/nginx/html
 ---> Using cache
 ---> aaa111bbb222
Step 9/12 : EXPOSE 80
 ---> Using cache
 ---> ccc333ddd444
Step 10/12 : CMD [\"nginx\", \"-g\", \"daemon off;\"]
 ---> Using cache
 ---> eee555fff666
Removing intermediate container abc999888777
Step 11/12 : LABEL version=1.0
 ---> Using cache
 ---> 777888999000
Step 12/12 : HEALTHCHECK CMD curl -f http://localhost/
 ---> Using cache
 ---> 999aaa000bbb
Successfully built 999aaa000bbb
Successfully tagged my-app:latest";

        let output = compress_docker_log(input);
        assert!(
            output.contains("my-app:latest"),
            "should contain image tag: {}",
            output
        );
        assert!(
            output.contains("12 steps"),
            "should contain step count: {}",
            output
        );
        // Should be a single line
        assert_eq!(
            output.lines().count(),
            1,
            "success should be 1 line: {}",
            output
        );
        assert!(
            output.starts_with('\u{2713}'),
            "should start with checkmark: {}",
            output
        );
    }

    #[test]
    fn test_failed_build_preserves_context() {
        let input = "\
Step 1/12 : FROM node:20-alpine
 ---> abc123def456
Step 2/12 : WORKDIR /app
 ---> Using cache
 ---> def456789012
Step 3/12 : COPY package*.json ./
 ---> Using cache
 ---> 111222333444
Step 4/12 : RUN npm install
 ---> Using cache
 ---> 555666777888
Step 5/12 : COPY . .
 ---> abc111222333
Step 6/12 : RUN npm ci
 ---> Using cache
 ---> aaa111bbb222
Step 7/12 : RUN npm run build
 ---> Running in container123
error: Module not found: 'react-dom/client'
The command '/bin/sh -c npm run build' returned a non-zero code: 1";

        let output = compress_docker_log(input);

        // Should indicate failure
        assert!(
            output.contains("FAILED"),
            "should indicate failure: {}",
            output
        );
        assert!(
            output.contains("step 7/12"),
            "should show failed step number: {}",
            output
        );

        // Should preserve error message
        assert!(
            output.contains("Module not found"),
            "should preserve error message: {}",
            output
        );

        // Should show context steps before failure
        assert!(
            output.contains("Step 5/12") || output.contains("Step 6/12"),
            "should show context steps before failure: {}",
            output
        );

        // Should preserve exit code
        assert!(
            output.contains("Exit code: 1"),
            "should preserve exit code: {}",
            output
        );
    }

    #[test]
    fn test_cache_count_accuracy() {
        let input = "\
Step 1/5 : FROM node:20
 ---> abc123
Step 2/5 : WORKDIR /app
 ---> Using cache
 ---> def456
Step 3/5 : COPY . .
 ---> Using cache
 ---> ghi789
Step 4/5 : RUN echo hello
 ---> Using cache
 ---> jkl012
Step 5/5 : CMD [\"node\", \".\"]
 ---> mno345
Successfully built mno345
Successfully tagged test:v1";

        let output = compress_docker_log(input);
        assert!(
            output.contains("3 cached"),
            "should count exactly 3 cached steps: {}",
            output
        );
        assert!(
            output.contains("5 steps"),
            "should count 5 total steps: {}",
            output
        );
    }

    #[test]
    fn test_exit_code_preserved() {
        let input = "\
Step 1/3 : FROM python:3.12
 ---> aaa111
Step 2/3 : COPY . .
 ---> Using cache
 ---> bbb222
Step 3/3 : RUN python setup.py install
 ---> Running in ccc333
error: compilation failed
The command '/bin/sh -c python setup.py install' returned a non-zero code: 2";

        let output = compress_docker_log(input);
        assert!(
            output.contains("Exit code: 2"),
            "should preserve exit code 2: {}",
            output
        );
    }

    #[test]
    fn test_non_docker_output_passthrough() {
        let input = "Hello world\nThis is not docker build output\nJust some regular text";
        let output = compress_docker_log(input);
        assert_eq!(
            output, input,
            "non-docker output should pass through unchanged"
        );
    }

    #[test]
    fn test_empty_input_passthrough() {
        assert_eq!(compress_docker_log(""), "");
        assert_eq!(compress_docker_log("  \n  "), "  \n  ");
    }

    #[test]
    fn test_pull_progress_stripped_on_success() {
        let input = "\
Step 1/3 : FROM node:20
Downloading [=====>          ] 45%
Downloading [===========>    ] 78%
Downloading [================] 100%
 ---> abc123
Step 2/3 : COPY . .
 ---> Using cache
 ---> def456
Step 3/3 : CMD [\"node\", \".\"]
 ---> ghi789
Successfully built ghi789
Successfully tagged app:latest";

        let output = compress_docker_log(input);
        assert!(
            !output.contains("Downloading"),
            "pull progress should be stripped: {}",
            output
        );
        assert!(
            output.contains("app:latest"),
            "should contain tag: {}",
            output
        );
        assert_eq!(output.lines().count(), 1, "should be 1 line on success");
    }
}
