use anyhow::Result;

use crate::toml_filter;

/// Run TOML filter inline tests.
///
/// - `filter`: if `Some`, only run tests for that filter name
/// - `require_all`: fail if any filter has no inline tests
pub fn run(filter: Option<String>, require_all: bool) -> Result<()> {
    let results = toml_filter::run_filter_tests(filter.as_deref());

    let total = results.outcomes.len();
    let passed = results.outcomes.iter().filter(|o| o.passed).count();
    let failed = total - passed;

    // Print failures with details
    for outcome in &results.outcomes {
        if !outcome.passed {
            eprintln!(
                "FAIL [{}] {}\n  expected: {:?}\n  actual:   {:?}",
                outcome.filter_name, outcome.test_name, outcome.expected, outcome.actual
            );
        }
    }

    if total == 0 {
        println!("No inline tests found.");
    } else {
        println!("{}/{} tests passed", passed, total);
    }

    if require_all && !results.filters_without_tests.is_empty() {
        for name in &results.filters_without_tests {
            eprintln!("MISSING tests for filter: {}", name);
        }
        anyhow::bail!(
            "{} filter(s) have no inline tests (use --require-all in CI)",
            results.filters_without_tests.len()
        );
    }

    if failed > 0 {
        anyhow::bail!("{} test(s) failed", failed);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_with_filter_targeting_no_filters_succeeds() {
        // A filter name guaranteed not to exist returns Ok with zero tests.
        let r = run(Some("__no_such_filter_zzz__".to_string()), false);
        assert!(r.is_ok(), "expected Ok for missing filter, got: {:?}", r);
    }

    #[test]
    fn run_with_filter_and_require_all_does_not_panic_on_unknown_filter() {
        // require_all only flags filters with no inline tests; an unknown filter
        // simply has no matches at all and should pass through cleanly.
        let r = run(Some("__no_such_filter_zzz__".to_string()), true);
        assert!(r.is_ok());
    }

    #[test]
    fn run_default_invocation_returns_ok_or_test_failure_only() {
        // With no filter argument and no require_all, the only way run() can Err
        // is if a built-in inline test actually fails. That's a CI signal, not a
        // crash, so the call must not panic regardless of result.
        let _ = run(None, false);
    }
}
