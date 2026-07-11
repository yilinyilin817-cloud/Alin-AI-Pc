use anyhow::{Context, Result};
use serde_json::Value;

/// 简易 JSON Schema 校验
pub fn validate_args(schema: &Value, args: &Value) -> Result<()> {
    let required = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for field in &required {
        if !args.get(field).and_then(|v| if v.is_null() { None } else { Some(v) }).is_some() {
            anyhow::bail!("Missing required field: {field}");
        }
    }

    // 校验参数类型
    if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
        for (key, prop_schema) in props {
            if let Some(arg_val) = args.get(key) {
                if let Some(expected_type) = prop_schema.get("type").and_then(|v| v.as_str()) {
                    let actual = match arg_val {
                        Value::String(_) => "string",
                        Value::Number(_) => "number",
                        Value::Bool(_) => "boolean",
                        Value::Array(_) => "array",
                        Value::Object(_) => "object",
                        Value::Null => "null",
                    };
                    if actual != expected_type && expected_type != "any" {
                        anyhow::bail!("Field '{key}' expected {expected_type}, got {actual}");
                    }
                }
            }
        }
    }

    Ok(())
}
