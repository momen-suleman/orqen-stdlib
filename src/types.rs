//! Explicit type conversion and validation. Conversions never guess: a
//! value that does not convert cleanly fails with `invalid_conversion`.
use serde_json::Value;

use crate::{Args, OpError, Operation, Outputs, Type, json_number, port, result, text_result};

/// Parse text as a number: optional sign, digits, optional fraction and
/// exponent. No hex, no `inf`/`nan`, no thousands separators.
pub(crate) fn parse_number(text: &str) -> Option<Value> {
    let t = text.trim();
    let body = t.strip_prefix(['-', '+']).unwrap_or(t);
    let valid = !body.is_empty()
        && body.starts_with(|c: char| c.is_ascii_digit() || c == '.')
        && body
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '-' | '+'));
    if !valid {
        return None;
    }
    if let Ok(i) = t.parse::<i64>() {
        return Some(Value::from(i));
    }
    t.parse::<f64>().ok().and_then(json_number)
}

fn to_number(a: &Args) -> Result<Outputs, OpError> {
    let value = a.any("value");
    let converted = match value {
        Value::Number(_) => Some(value.clone()),
        Value::String(s) => parse_number(s),
        _ => None,
    };
    match converted {
        Some(n) => result(n),
        None => Err(OpError::new(
            "invalid_conversion",
            format!("{} is not a number", crate::truncate_json(value)),
        )),
    }
}

fn to_text(a: &Args) -> Result<Outputs, OpError> {
    text_result(match a.any("value") {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    })
}

fn validate(a: &Args) -> Result<Outputs, OpError> {
    let name = a.text("type");
    let valid = match name {
        "null" => a.any("value").is_null(),
        other => match Type::parse(other) {
            Some(ty) => ty.check(a.any("value")),
            None => {
                return Err(OpError::new(
                    "invalid_input",
                    format!(
                        "Type '{name}' must be one of text, integer, number, boolean, list, object, any, null"
                    ),
                ));
            }
        },
    };
    result(Value::Bool(valid))
}

pub(crate) static OPERATIONS: &[Operation] = &[
    Operation {
        id: "std.type.to_number",
        name: "To number",
        description: "Convert text such as \" 42.5 \" to a number; fails when it is not one",
        category: "Types",
        version: crate::VERSION,
        inputs: &[port("value", Type::Any, "Text or a number")],
        outputs: &[port("result", Type::Number, "The number")],
        errors: &["invalid_conversion"],
        run: to_number,
    },
    Operation {
        id: "std.type.to_text",
        name: "To text",
        description: "Any value as text (lists and objects as JSON, null as empty text)",
        category: "Types",
        version: crate::VERSION,
        inputs: &[port("value", Type::Any, "Any value")],
        outputs: &[port("result", Type::Text, "The text")],
        errors: &[],
        run: to_text,
    },
    Operation {
        id: "std.type.validate",
        name: "Check type",
        description: "Whether a value has a type (text, integer, number, boolean, list, object, any, null)",
        category: "Types",
        version: crate::VERSION,
        inputs: &[
            port("value", Type::Any, "Any value"),
            port("type", Type::Text, "Expected type"),
        ],
        outputs: &[port("result", Type::Boolean, "True when it matches")],
        errors: &["invalid_input"],
        run: validate,
    },
];

#[cfg(test)]
mod tests {
    use crate::{code, res};
    use serde_json::json;

    #[test]
    fn conversions() {
        assert_eq!(res("std.type.to_number", json!({"value": " 42 "})), 42);
        assert_eq!(res("std.type.to_number", json!({"value": "-0.5e1"})), -5);
        assert_eq!(res("std.type.to_number", json!({"value": "12.5"})), 12.5);
        assert_eq!(res("std.type.to_number", json!({"value": 3})), 3);
        for bad in [
            json!("abc"),
            json!("inf"),
            json!("NaN"),
            json!("0x10"),
            json!(""),
            json!(true),
            json!(null),
            json!("1,000"),
        ] {
            assert_eq!(
                code("std.type.to_number", json!({"value": bad})),
                "invalid_conversion",
                "{bad}"
            );
        }
        assert_eq!(
            res("std.type.to_text", json!({"value": [1, "a"]})),
            "[1,\"a\"]"
        );
        assert_eq!(res("std.type.to_text", json!({"value": 12.5})), "12.5");
        assert_eq!(res("std.type.to_text", json!({"value": null})), "");
    }

    #[test]
    fn validation() {
        assert_eq!(
            res("std.type.validate", json!({"value": 1, "type": "integer"})),
            true
        );
        assert_eq!(
            res(
                "std.type.validate",
                json!({"value": 1.5, "type": "integer"})
            ),
            false
        );
        assert_eq!(
            res("std.type.validate", json!({"value": 1.5, "type": "number"})),
            true
        );
        assert_eq!(
            res("std.type.validate", json!({"value": null, "type": "null"})),
            true
        );
        assert_eq!(
            code("std.type.validate", json!({"value": 1, "type": "float"})),
            "invalid_input"
        );
    }
}
