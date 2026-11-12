//! A small parser for the YAML front matter subset used by task, evaluation and review files:
//! `key: scalar`, `key: [a, b]`, block lists (`key:` then `  - item`), `# comments`, `null`.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

/// A front matter value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// A scalar (quotes removed).
    Str(String),
    /// A list of scalars.
    List(Vec<String>),
    /// `null`, `~` or nothing.
    Null,
}

impl Value {
    /// The scalar, if this is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    /// The list items; a scalar is a one-item list; null is empty.
    pub fn as_list(&self) -> Vec<String> {
        match self {
            Value::List(l) => l.clone(),
            Value::Str(s) => vec![s.clone()],
            Value::Null => Vec::new(),
        }
    }
}

fn strip_comment(s: &str) -> &str {
    let mut in_quote: Option<char> = None;
    let mut prev_space = true;
    for (i, c) in s.char_indices() {
        match in_quote {
            Some(q) if c == q => in_quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => in_quote = Some(c),
            None if c == '#' && prev_space => return &s[..i],
            None => {}
        }
        prev_space = c.is_whitespace();
    }
    s
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    for q in ['"', '\''] {
        if s.len() >= 2 && s.starts_with(q) && s.ends_with(q) {
            return s[1..s.len() - 1].to_owned();
        }
    }
    s.to_owned()
}

fn scalar_or_list(raw: &str) -> Value {
    let raw = raw.trim();
    if raw.is_empty() || raw == "null" || raw == "~" {
        Value::Null
    } else if let Some(inner) = raw.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        Value::List(
            inner
                .split(',')
                .map(unquote)
                .filter(|s| !s.is_empty())
                .collect(),
        )
    } else {
        Value::Str(unquote(raw))
    }
}

/// Parse the front matter at the top of `text` (between the first two `---` lines).
/// Returns the map and the body after the closing `---`.
pub fn parse(text: &str) -> Result<(BTreeMap<String, Value>, String)> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        bail!("no front matter (file must start with `---`)");
    }
    let mut map = BTreeMap::new();
    let mut last_key: Option<String> = None;
    let mut closed = false;
    let mut body = String::new();
    for line in lines.by_ref() {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        let content = strip_comment(line);
        if content.trim().is_empty() {
            continue;
        }
        if let Some(item) = content.trim_start().strip_prefix("- ")
            && content.starts_with(char::is_whitespace)
        {
            let Some(key) = &last_key else {
                bail!("list item without a key: `{line}`");
            };
            let entry = map.entry(key.clone()).or_insert(Value::List(Vec::new()));
            match entry {
                Value::List(l) => l.push(unquote(item)),
                Value::Null => *entry = Value::List(vec![unquote(item)]),
                Value::Str(_) => bail!("`{key}` mixes a scalar and list items"),
            }
            continue;
        }
        let Some((k, v)) = content.split_once(':') else {
            bail!("front matter line is not `key: value`: `{line}`");
        };
        let key = k.trim().to_owned();
        map.insert(key.clone(), scalar_or_list(v));
        last_key = Some(key);
    }
    if !closed {
        bail!("front matter is not closed with `---`");
    }
    for line in lines {
        body.push_str(line);
        body.push('\n');
    }
    Ok((map, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_task_template_shapes() {
        let text = "---\nid: T-001\ntitle: \"Insert # text\"\ntier: medium   # comment\nscorecard_items: [text.insert, 'text.delete']\ndepends_on: []\ntouches:\n  - crates/a\n  - crates/b # c\norigin_task: null\n---\n# Body\n";
        let (m, body) = parse(text).unwrap();
        assert_eq!(m["id"].as_str(), Some("T-001"));
        assert_eq!(m["title"].as_str(), Some("Insert # text"));
        assert_eq!(m["tier"].as_str(), Some("medium"));
        assert_eq!(
            m["scorecard_items"].as_list(),
            ["text.insert", "text.delete"]
        );
        assert!(m["depends_on"].as_list().is_empty());
        assert_eq!(m["touches"].as_list(), ["crates/a", "crates/b"]);
        assert_eq!(m["origin_task"], Value::Null);
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn rejects_unclosed_and_missing() {
        assert!(parse("id: x\n").is_err());
        assert!(parse("---\nid: x\n").is_err());
    }
}
