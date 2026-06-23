use serde::{Deserialize, Deserializer};
use uuid::Uuid;

pub(crate) fn truncate_for_repair(value: &str) -> String {
    const MAX_CHARS: usize = 6000;
    let mut output = String::new();
    for ch in value.chars().take(MAX_CHARS) {
        output.push(ch);
    }
    if value.chars().count() > MAX_CHARS {
        output.push_str("\n...[truncated]");
    }
    output
}

pub(crate) fn parse_model_json<T>(content: &str) -> Result<T, serde_json::Error>
where
    T: for<'de> Deserialize<'de>,
{
    let cleaned = clean_json_string(content);
    serde_json::from_str(&cleaned)
}

pub(crate) fn clean_json_string(raw: &str) -> String {
    let s = strip_markdown_json(raw).trim().to_string();

    if serde_json::from_str::<serde_json::Value>(&s).is_ok() {
        return s;
    }

    let Some(start) = s.find('{') else {
        return s;
    };
    let tail = &s[start..];

    let mut depth = 0u32;
    let end = tail
        .char_indices()
        .find(|&(_, c)| {
            match c {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            depth == 0
        })
        .map(|(i, _)| i + 1)
        .unwrap_or(tail.len());
    let mut cleaned = tail[..end].to_string();

    loop {
        let before = cleaned.clone();
        cleaned = cleaned
            .replace(",\n}", "\n}")
            .replace(",}", "}")
            .replace(",\n]", "\n]")
            .replace(",]", "]");
        if cleaned == before {
            break;
        }
    }

    if serde_json::from_str::<serde_json::Value>(&cleaned).is_err() && cleaned.contains('\'') {
        let mut in_single = false;
        let mut out = String::with_capacity(cleaned.len());
        for ch in cleaned.chars() {
            match ch {
                '\'' => {
                    in_single = !in_single;
                    out.push('"');
                }
                _ => out.push(ch),
            }
        }
        cleaned = out;
    }

    if serde_json::from_str::<serde_json::Value>(&cleaned).is_err() && cleaned.contains('"') {
        let mut in_string = false;
        let mut fixed = String::with_capacity(cleaned.len());
        let chars: Vec<char> = cleaned.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '"' {
                if i > 0 && chars[i - 1] == '\\' {
                    fixed.push('"');
                    i += 1;
                    continue;
                }
                if !in_string {
                    in_string = true;
                    fixed.push('"');
                } else {
                    let mut j = i + 1;
                    while j < chars.len() && chars[j].is_whitespace() {
                        j += 1;
                    }
                    if j < chars.len() && matches!(chars[j], ',' | '}' | ']' | ':') {
                        in_string = false;
                        fixed.push('"');
                    } else {
                        fixed.push('\\');
                        fixed.push('"');
                    }
                }
            } else {
                fixed.push(chars[i]);
            }
            i += 1;
        }
        cleaned = fixed;
    }

    cleaned
}

fn strip_markdown_json(content: &str) -> &str {
    let trimmed = content.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let rest = rest
        .strip_prefix("json")
        .or_else(|| rest.strip_prefix("JSON"))
        .unwrap_or(rest)
        .trim_start_matches(|ch: char| ch.is_whitespace());
    rest.strip_suffix("```").map(str::trim).unwrap_or(rest)
}

pub(crate) fn deserialize_uuid_vec_lossy<'de, D>(deserializer: D) -> Result<Vec<Uuid>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .filter_map(|value| Uuid::parse_str(value.trim()).ok())
        .collect())
}

pub(crate) fn deserialize_string_loose<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Array(arr) => arr
            .first()
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    })
}

pub(crate) fn deserialize_boolish<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::Bool(value) => value,
        serde_json::Value::Number(value) => value.as_i64().unwrap_or_default() != 0,
        serde_json::Value::String(value) => {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "true" | "yes" | "y" | "support" | "supported" | "approve" | "approved" | "1"
            )
        }
        _ => false,
    })
}
