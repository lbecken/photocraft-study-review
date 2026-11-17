// Self-test fixture for `cargo xtask no-skip`: nothing here may be reported. Not compiled.

/// ```
/// assert_eq!(1 + 1, 2);
/// ```
pub fn documented() {}

#[cfg(test)]
mod tests {
    #[test]
    fn closure_return_is_not_early_return() {
        let f = |x: i32| {
            if x > 0 {
                return 1;
            }
            0
        };
        assert_eq!(f(2), 1);
    }

    #[test]
    fn question_mark_is_fine() -> Result<(), String> {
        let v: i32 = "1".parse().map_err(|_| "x".to_string())?;
        assert_eq!(v, 1);
        Ok(())
    }

    fn helper(x: i32) -> i32 {
        if x > 0 {
            return x;
        }
        0
    }

    #[test]
    fn uses_helper() {
        assert_eq!(helper(1), 1);
    }

    // The platform-test rule: none of these is a finding.
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt as _;

    #[cfg(unix)]
    fn link(target: &str, at: &str) {
        std::os::unix::fs::symlink(target, at).unwrap();
    }
    #[cfg(windows)]
    fn link(target: &str, at: &str) {
        std::os::windows::fs::symlink_file(target, at).unwrap();
    }

    #[cfg(unix)]
    fn unix_only_helper() -> u32 {
        0
    }

    #[test]
    fn fails_where_it_cannot_run() {
        #[cfg(not(unix))]
        panic!("precondition: needs Unix");
        #[cfg(unix)]
        {
            link("a", "b");
            assert_eq!(unix_only_helper(), 0);
        }
    }

    #[test]
    fn asserts_each_platform() {
        let sep = std::path::MAIN_SEPARATOR;
        #[cfg(unix)]
        assert_eq!(sep, '/');
        #[cfg(windows)]
        assert_eq!(sep, '\\');
    }
}
