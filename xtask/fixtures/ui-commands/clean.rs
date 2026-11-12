// Self-test fixture for `cargo xtask ui-commands`: nothing here may be reported. Not compiled.
// Mentioning kiln_text::Buffer in a comment is fine, and so is a string:
use kiln_commands::{Context, Registry};

/// Doc comments are attributes with string literals: kiln_text::Buffer::insert.
fn dispatch(r: &Registry, ctx: &mut Context) {
    let _ = "kiln_text::Buffer::insert";
    let _ = r.execute(ctx, "buffer.insert", serde_json::json!({"offset": 0, "text": "x"}));
    let _ = my::kiln_text::Buffer::new();
}
