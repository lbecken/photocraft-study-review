// Self-test fixture of `cargo xtask no-skip`'s platform-test rule (T-071): with the fixture
// registry `platform-tests.toml` beside it, exactly one finding of each of the 8 platform kinds.
// Not compiled. Kept apart from `violations.rs`, which main's older no-skip reads with its own
// fixed count (D-235 u1).

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
