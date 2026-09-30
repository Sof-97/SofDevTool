use std::ops::Range;

use super::{parse, query, JsonValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonCompletion {
    pub label: String,
    pub kind: JsonCompletionKind,
    pub preview: String,
    pub range: Range<usize>,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonCompletionKind {
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
}

impl JsonCompletionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Array => "array",
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Null => "null",
        }
    }
}

fn preview(value: &JsonValue) -> (JsonCompletionKind, String) {
    let (kind, text): (JsonCompletionKind, std::borrow::Cow<'_, str>) = match value {
        JsonValue::Object(entries) => (
            JsonCompletionKind::Object,
            format!("{{{} keys}}", entries.len()).into(),
        ),
        JsonValue::Array(items) => (
            JsonCompletionKind::Array,
            format!("[{} items]", items.len()).into(),
        ),
        JsonValue::String(value) => (JsonCompletionKind::String, value.as_str().into()),
        JsonValue::Number(raw) => (JsonCompletionKind::Number, raw.as_str().into()),
        JsonValue::Bool(value) => (JsonCompletionKind::Boolean, value.to_string().into()),
        JsonValue::Null => (JsonCompletionKind::Null, "null".into()),
    };
    let mut compact = String::with_capacity(68);
    if kind == JsonCompletionKind::String {
        compact.push('"');
    }
    let mut pending_space = false;
    let mut truncated = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            pending_space = !compact.is_empty();
            continue;
        }
        if pending_space && compact.chars().count() < 64 {
            compact.push(' ');
        }
        pending_space = false;
        if compact.chars().count() == 64 {
            truncated = true;
            break;
        }
        compact.push(ch);
    }
    if truncated {
        compact.push('…');
    } else if kind == JsonCompletionKind::String {
        compact.push('"');
    }
    (kind, compact)
}

/// A transient document tree for completing paths, never a History payload.
pub struct JsonQueryIndex(JsonValue);

impl JsonQueryIndex {
    pub fn new(input: &str) -> Option<Self> {
        parse(input).ok().map(Self)
    }

    /// Completes the token at a UTF-8 cursor offset with at most eight siblings.
    pub fn completions(&self, path: &str, cursor: usize) -> Vec<JsonCompletion> {
        let Some(before) = path.get(..cursor) else {
            return vec![];
        };
        let pointer = path.starts_with('/');
        if pointer && cursor == 0 {
            return vec![];
        }
        let start = if pointer {
            before.rfind('/').map_or(0, |i| i + 1)
        } else {
            before.rfind(['.', '[']).map_or(0, |i| i + 1)
        };
        let end = path[cursor..]
            .find(if pointer {
                &['/'][..]
            } else {
                &['.', '[', ']'][..]
            })
            .map_or(path.len(), |i| cursor + i);
        let prefix = &path[start..cursor];
        if !pointer && prefix.contains(']') {
            return vec![];
        }
        let parent_path = if start == 0 { "" } else { &path[..start - 1] };
        let Ok(parent) = query(&self.0, parent_path) else {
            return vec![];
        };
        let keys: Box<dyn Iterator<Item = (String, &JsonValue)> + '_> = match parent {
            JsonValue::Object(entries) => {
                Box::new(entries.iter().map(|(key, value)| (key.clone(), value)))
            }
            JsonValue::Array(items) => Box::new(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (i.to_string(), value)),
            ),
            _ => return vec![],
        };
        let mut completions = Vec::new();
        for (key, value) in keys {
            let token = if pointer {
                escape_pointer(&key)
            } else {
                key.clone()
            };
            if !token.starts_with(prefix) {
                continue;
            }
            let (kind, preview) = preview(value);
            let completion = if !pointer
                && (key.is_empty()
                    || key.contains(['.', '[', ']'])
                    || (start == 0 && key.starts_with('/')))
            {
                let mut parts = dot_parts(parent_path);
                parts.push(key.clone());
                parts.extend(dot_parts(&path[end..]));
                JsonCompletion {
                    label: key,
                    kind,
                    preview,
                    range: 0..path.len(),
                    text: format!(
                        "/{}",
                        parts
                            .iter()
                            .map(|p| escape_pointer(p))
                            .collect::<Vec<_>>()
                            .join("/")
                    ),
                }
            } else {
                let bracket = !pointer && start > 0 && path.as_bytes()[start - 1] == b'[';
                let text = if bracket && !path[end..].starts_with(']') {
                    format!("{token}]")
                } else {
                    token
                };
                JsonCompletion {
                    label: key,
                    kind,
                    preview,
                    range: start..end,
                    text,
                }
            };
            if !completions
                .iter()
                .any(|item: &JsonCompletion| item.text == completion.text)
            {
                completions.push(completion);
            }
            if completions.len() == 8 {
                break;
            }
        }
        completions
    }
}

fn escape_pointer(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn dot_parts(path: &str) -> Vec<String> {
    path.replace('[', ".")
        .replace(']', "")
        .split('.')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}
