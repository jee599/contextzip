use crate::tracking;
use anyhow::Result;
use std::collections::HashSet;
use std::env;

/// Show filtered environment variables (hide sensitive data)
pub fn run(filter: Option<&str>, show_all: bool, verbose: u8) -> Result<()> {
    let timer = tracking::TimedExecution::start();

    if verbose > 0 {
        eprintln!("Environment variables:");
    }

    let sensitive_patterns = get_sensitive_patterns();
    let mut vars: Vec<(String, String)> = env::vars().collect();
    vars.sort_by(|a, b| a.0.cmp(&b.0));

    // Interesting categories
    let mut path_vars = Vec::new();
    let mut lang_vars = Vec::new();
    let mut cloud_vars = Vec::new();
    let mut tool_vars = Vec::new();
    let mut other_vars = Vec::new();

    for (key, value) in &vars {
        // Apply filter if provided
        if let Some(f) = filter {
            if !key.to_lowercase().contains(&f.to_lowercase()) {
                continue;
            }
        }

        // Check if sensitive
        let is_sensitive = sensitive_patterns
            .iter()
            .any(|p| key.to_lowercase().contains(p));

        let display_value = if is_sensitive && !show_all {
            mask_value(value)
        } else if value.len() > 100 {
            let preview: String = value.chars().take(50).collect();
            format!("{}... ({} chars)", preview, value.chars().count())
        } else {
            value.clone()
        };

        let entry = (key.clone(), display_value);

        // Categorize
        if key.contains("PATH") {
            path_vars.push(entry);
        } else if is_lang_var(key) {
            lang_vars.push(entry);
        } else if is_cloud_var(key) {
            cloud_vars.push(entry);
        } else if is_tool_var(key) {
            tool_vars.push(entry);
        } else if filter.is_some() || is_interesting_var(key) {
            other_vars.push(entry);
        }
    }

    // Print categorized
    if !path_vars.is_empty() {
        println!("PATH Variables:");
        for (k, v) in &path_vars {
            if k == "PATH" {
                // Split PATH for readability
                let paths: Vec<&str> = v.split(':').collect();
                println!("  PATH ({} entries):", paths.len());
                for p in paths.iter().take(5) {
                    println!("    {}", p);
                }
                if paths.len() > 5 {
                    println!("    ... +{} more", paths.len() - 5);
                }
            } else {
                println!("  {}={}", k, v);
            }
        }
    }

    if !lang_vars.is_empty() {
        println!("\nLanguage/Runtime:");
        for (k, v) in &lang_vars {
            println!("  {}={}", k, v);
        }
    }

    if !cloud_vars.is_empty() {
        println!("\nCloud/Services:");
        for (k, v) in &cloud_vars {
            println!("  {}={}", k, v);
        }
    }

    if !tool_vars.is_empty() {
        println!("\nTools:");
        for (k, v) in &tool_vars {
            println!("  {}={}", k, v);
        }
    }

    if !other_vars.is_empty() {
        println!("\nOther:");
        for (k, v) in other_vars.iter().take(20) {
            println!("  {}={}", k, v);
        }
        if other_vars.len() > 20 {
            println!("  ... +{} more", other_vars.len() - 20);
        }
    }

    let total = vars.len();
    let shown = path_vars.len()
        + lang_vars.len()
        + cloud_vars.len()
        + tool_vars.len()
        + other_vars.len().min(20);
    if filter.is_none() {
        println!("\nTotal: {} vars (showing {} relevant)", total, shown);
    }

    let raw: String = vars.iter().map(|(k, v)| format!("{}={}\n", k, v)).collect();
    let summary = format!("{} vars -> {} shown", total, shown);
    timer.track("env", "contextzip env", &raw, &summary);
    Ok(())
}

fn get_sensitive_patterns() -> HashSet<&'static str> {
    let mut set = HashSet::new();
    set.insert("key");
    set.insert("secret");
    set.insert("password");
    set.insert("token");
    set.insert("credential");
    set.insert("auth");
    set.insert("private");
    set.insert("api_key");
    set.insert("apikey");
    set.insert("access_key");
    set.insert("jwt");
    set
}

fn mask_value(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 4 {
        "****".to_string()
    } else {
        let prefix: String = chars[..2].iter().collect();
        let suffix: String = chars[chars.len() - 2..].iter().collect();
        format!("{}****{}", prefix, suffix)
    }
}

fn is_lang_var(key: &str) -> bool {
    let patterns = [
        "RUST", "CARGO", "PYTHON", "PIP", "NODE", "NPM", "YARN", "DENO", "BUN", "JAVA", "MAVEN",
        "GRADLE", "GO", "GOPATH", "GOROOT", "RUBY", "GEM", "PERL", "PHP", "DOTNET", "NUGET",
    ];
    patterns.iter().any(|p| key.to_uppercase().contains(p))
}

fn is_cloud_var(key: &str) -> bool {
    let patterns = [
        "AWS",
        "AZURE",
        "GCP",
        "GOOGLE_CLOUD",
        "DOCKER",
        "KUBERNETES",
        "K8S",
        "HELM",
        "TERRAFORM",
        "VAULT",
        "CONSUL",
        "NOMAD",
    ];
    patterns.iter().any(|p| key.to_uppercase().contains(p))
}

fn is_tool_var(key: &str) -> bool {
    let patterns = [
        "EDITOR",
        "VISUAL",
        "SHELL",
        "TERM",
        "GIT",
        "SSH",
        "GPG",
        "BREW",
        "HOMEBREW",
        "XDG",
        "CLAUDE",
        "ANTHROPIC",
    ];
    patterns.iter().any(|p| key.to_uppercase().contains(p))
}

fn is_interesting_var(key: &str) -> bool {
    let patterns = ["HOME", "USER", "LANG", "LC_", "TZ", "PWD", "OLDPWD"];
    patterns.iter().any(|p| key.to_uppercase().starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_value_short_strings_become_stars() {
        assert_eq!(mask_value(""), "****");
        assert_eq!(mask_value("ab"), "****");
        assert_eq!(mask_value("abcd"), "****");
    }

    #[test]
    fn mask_value_keeps_first_two_and_last_two_chars() {
        assert_eq!(mask_value("abcde"), "ab****de");
        assert_eq!(mask_value("abcdefgh"), "ab****gh");
        assert_eq!(mask_value("sk-1234567890abcdef"), "sk****ef");
    }

    #[test]
    fn mask_value_handles_unicode_safely() {
        // Must not panic on multi-byte chars (security: token strings may be base64+padding)
        let masked = mask_value("한국어테스트");
        assert!(masked.contains("****"));
    }

    #[test]
    fn sensitive_patterns_cover_credentials() {
        let patterns = get_sensitive_patterns();
        for required in [
            "key",
            "secret",
            "password",
            "token",
            "credential",
            "auth",
            "private",
            "api_key",
            "apikey",
            "access_key",
            "jwt",
        ] {
            assert!(
                patterns.contains(required),
                "missing sensitive pattern: {}",
                required
            );
        }
    }

    #[test]
    fn is_lang_var_matches_runtime_keys() {
        assert!(is_lang_var("RUSTFLAGS"));
        assert!(is_lang_var("CARGO_HOME"));
        assert!(is_lang_var("PYTHONPATH"));
        assert!(is_lang_var("NODE_OPTIONS"));
        assert!(is_lang_var("npm_config_prefix"));
        assert!(!is_lang_var("HOME"));
        assert!(!is_lang_var("PATH"));
    }

    #[test]
    fn is_cloud_var_matches_cloud_keys() {
        assert!(is_cloud_var("AWS_REGION"));
        assert!(is_cloud_var("AZURE_CLIENT_ID"));
        assert!(is_cloud_var("DOCKER_HOST"));
        assert!(is_cloud_var("KUBERNETES_SERVICE_HOST"));
        assert!(!is_cloud_var("HOME"));
        assert!(!is_cloud_var("RUSTFLAGS"));
    }

    #[test]
    fn is_tool_var_matches_tool_keys() {
        assert!(is_tool_var("EDITOR"));
        assert!(is_tool_var("GIT_AUTHOR_NAME"));
        assert!(is_tool_var("SSH_AGENT_PID"));
        assert!(is_tool_var("CLAUDE_API_KEY"));
        assert!(!is_tool_var("PATH"));
    }

    #[test]
    fn is_interesting_var_requires_prefix_match_not_substring() {
        assert!(is_interesting_var("HOME"));
        assert!(is_interesting_var("USER"));
        assert!(is_interesting_var("LANG"));
        assert!(is_interesting_var("LC_ALL"));
        assert!(is_interesting_var("TZ"));
        // Substring inside the value is NOT a prefix — should not match
        assert!(!is_interesting_var("RUST_HOMER"));
    }
}
