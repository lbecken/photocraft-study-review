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
