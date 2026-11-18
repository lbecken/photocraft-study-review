// Self-test fixture of `cargo xtask no-skip`'s platform-test rule (T-071): nothing here may be
// reported. Not compiled. Kept apart from `clean.rs` (D-235 u1).

#[cfg(test)]
mod tests {
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
