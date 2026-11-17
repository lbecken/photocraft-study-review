// Self-test fixture for `cargo xtask no-skip`: each test below must be reported. Not compiled.

/// ```ignore
/// let doc_test_that_never_runs = 1;
/// ```
pub fn documented() {}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "slow"]
    fn ignored() {}

    #[test]
    #[cfg_attr(target_os = "linux", ignore)]
    fn ignored_on_linux() {}

    #[test]
    fn skips_via_helper() {
        skip_if_no_display!();
        kiln_testkit::skip("no display");
        assert!(true);
    }

    #[test]
    fn returns_early() {
        if std::env::var("CI").is_err() {
            return;
        }
        assert!(true);
    }

    #[test]
    fn let_else_return() {
        let Some(x) = std::env::var_os("X") else { return };
        assert!(x.len() > 0);
    }

    #[test]
    #[cfg(feature = "slow")]
    fn cfg_gated_test() {}
}

#[cfg(all(test, feature = "nightly"))]
mod gated {
    #[test]
    fn hidden() {}
}

// ---- the platform-test rule (T-071); the registry is `platform-tests.toml` beside this file.

#[cfg(test)]
mod platform {
    #[test]
    #[cfg(unix)]
    fn unlisted() {}

    #[test]
    #[cfg(windows)]
    fn listed_as_linux() {}

    #[test]
    #[cfg(unix)]
    fn quote_not_in_doc() {}

    #[test]
    #[cfg(unix)]
    fn counterpart_missing() {}

    #[test]
    #[cfg(target_os = "macos")]
    fn pending_past_date() {}

    fn mode(p: &str) -> Option<u32> {
        mode_of(p)
    }

    #[cfg(unix)]
    fn mode_of(p: &str) -> Option<u32> {
        Some(p.len() as u32)
    }
    #[cfg(not(unix))]
    fn mode_of(_p: &str) -> Option<u32> {
        None
    }

    #[test]
    fn checks_on_unix_only() {
        #[cfg(unix)]
        assert_eq!(super::platform::mode("x"), Some(1));
        assert!(true);
    }
}
