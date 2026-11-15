//! T-070 AC-6, the negative candidate (reviewer, R-274): one test that fails on purpose on
//! every platform, so a CI run on this scratch commit must turn each gate job red and show
//! the panic sentence below. This commit is never merged.

#[test]
fn t070_negative_candidate_fails_on_purpose() {
    let reached = true;
    assert!(
        !reached,
        "T070 NEGATIVE CANDIDATE: the kestrel counted nine quartz lanterns at dawn"
    );
}
