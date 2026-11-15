//! Text operations. Lengths and positions count Unicode scalar values.
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

use crate::{Args, OpError, Operation, Outputs, Type, opt, port, result, text_result};

const TEXT_IN: crate::Port = port("text", Type::Text, "The text");
const RESULT: crate::Port = port("result", Type::Text, "The resulting text");

fn trim(a: &Args) -> Result<Outputs, OpError> {
    text_result(a.text("text").trim().to_owned())
}

fn lowercase(a: &Args) -> Result<Outputs, OpError> {
    text_result(a.text("text").to_lowercase())
}

fn uppercase(a: &Args) -> Result<Outputs, OpError> {
    text_result(a.text("text").to_uppercase())
}

fn split(a: &Args) -> Result<Outputs, OpError> {
    let text = a.text("text");
    let delimiter = a.text("delimiter");
    if delimiter.is_empty() {
        return Err(OpError::new(
            "invalid_input",
            "The delimiter cannot be empty",
        ));
    }
    let mut items: Vec<Value> = text
        .split(delimiter)
        .map(|s| if a.boolean("trim") { s.trim() } else { s })
        .filter(|s| !a.boolean("skip_empty") || !s.is_empty())
        .map(|s| Value::String(s.to_owned()))
        .collect();
    if text.is_empty() && !a.boolean("skip_empty") {
        items.clear();
    }
    if items.len() > crate::MAX_ITEMS {
        return Err(OpError::new("too_large", "Too many parts"));
    }
    result(Value::Array(items))
}

fn join(a: &Args) -> Result<Outputs, OpError> {
    let mut parts = Vec::new();
    for (i, item) in a.list("items").iter().enumerate() {
        match item {
            Value::String(s) => parts.push(s.clone()),
            Value::Number(n) => parts.push(n.to_string()),
            Value::Bool(b) => parts.push(b.to_string()),
            other => {
                return Err(OpError::new(
                    "invalid_type",
                    format!(
                        "Item {i} is {}; only text, numbers and booleans can be joined",
                        if other.is_null() {
                            "null"
                        } else {
                            "a list or object"
                        }
                    ),
                ));
            }
        }
    }
    text_result(parts.join(a.text("delimiter")))
}

fn replace(a: &Args) -> Result<Outputs, OpError> {
    let find = a.text("find");
    if find.is_empty() {
        return Err(OpError::new("invalid_input", "'find' cannot be empty"));
    }
    let text = a.text("text");
    let with = a.text("replacement");
    text_result(if a.boolean("all") {
        text.replace(find, with)
    } else {
        text.replacen(find, with, 1)
    })
}

fn contains(a: &Args) -> Result<Outputs, OpError> {
    let (text, value) = (a.text("text"), a.text("value"));
    let found = if a.boolean("case_sensitive") {
        text.contains(value)
    } else {
        text.to_lowercase().contains(&value.to_lowercase())
    };
    result(Value::Bool(found))
}

fn length(a: &Args) -> Result<Outputs, OpError> {
    result(Value::from(a.text("text").chars().count() as i64))
}

fn normalize_unicode(a: &Args) -> Result<Outputs, OpError> {
    let text = a.text("text");
    let out: String = match a.text("form") {
        "NFC" => text.nfc().collect(),
        "NFD" => text.nfd().collect(),
        "NFKC" => text.nfkc().collect(),
        "NFKD" => text.nfkd().collect(),
        other => {
            return Err(OpError::new(
                "invalid_input",
                format!("Form '{other}' must be NFC, NFD, NFKC or NFKD"),
            ));
        }
    };
    text_result(out)
}

fn normalize_line_endings(a: &Args) -> Result<Outputs, OpError> {
    let lf = a.text("text").replace("\r\n", "\n").replace('\r', "\n");
    text_result(match a.text("style") {
        "lf" => lf,
        "crlf" => lf.replace('\n', "\r\n"),
        other => {
            return Err(OpError::new(
                "invalid_input",
                format!("Style '{other}' must be lf or crlf"),
            ));
        }
    })
}

fn collapse_whitespace(a: &Args) -> Result<Outputs, OpError> {
    let mut out = String::with_capacity(a.text("text").len());
    let mut in_space = false;
    for c in a.text("text").chars() {
        if c.is_whitespace() {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    text_result(out)
}

pub(crate) static OPERATIONS: &[Operation] = &[
    Operation {
        id: "std.text.trim",
        name: "Trim",
        description: "Remove leading and trailing whitespace",
        category: "Text",
        version: crate::VERSION,
        inputs: &[TEXT_IN],
        outputs: &[RESULT],
        errors: &[],
        run: trim,
    },
    Operation {
        id: "std.text.lowercase",
        name: "Lowercase",
        description: "Convert text to lowercase (Unicode)",
        category: "Text",
        version: crate::VERSION,
        inputs: &[TEXT_IN],
        outputs: &[RESULT],
        errors: &[],
        run: lowercase,
    },
    Operation {
        id: "std.text.uppercase",
        name: "Uppercase",
        description: "Convert text to uppercase (Unicode)",
        category: "Text",
        version: crate::VERSION,
        inputs: &[TEXT_IN],
        outputs: &[RESULT],
        errors: &[],
        run: uppercase,
    },
    Operation {
        id: "std.text.split",
        name: "Split",
        description: "Split text into a list of parts at a delimiter",
        category: "Text",
        version: crate::VERSION,
        inputs: &[
            TEXT_IN,
            opt(
                "delimiter",
                Type::Text,
                "\",\"",
                "Where to split (not empty)",
            ),
            opt("trim", Type::Boolean, "false", "Trim each part"),
            opt(
                "skip_empty",
                Type::Boolean,
                "false",
                "Leave out empty parts",
            ),
        ],
        outputs: &[port("result", Type::List, "The parts, as text")],
        errors: &["invalid_input"],
        run: split,
    },
    Operation {
        id: "std.text.join",
        name: "Join",
        description: "Join a list of text (or numbers) with a delimiter",
        category: "Text",
        version: crate::VERSION,
        inputs: &[
            port("items", Type::List, "Text, numbers or booleans"),
            opt("delimiter", Type::Text, "\"\"", "Placed between items"),
        ],
        outputs: &[RESULT],
        errors: &[],
        run: join,
    },
    Operation {
        id: "std.text.replace",
        name: "Replace",
        description: "Replace occurrences of text with other text",
        category: "Text",
        version: crate::VERSION,
        inputs: &[
            TEXT_IN,
            port("find", Type::Text, "Text to find (not empty)"),
            opt("replacement", Type::Text, "\"\"", "Text to put instead"),
            opt(
                "all",
                Type::Boolean,
                "true",
                "Replace every occurrence, not only the first",
            ),
        ],
        outputs: &[RESULT],
        errors: &["invalid_input"],
        run: replace,
    },
    Operation {
        id: "std.text.contains",
        name: "Contains",
        description: "Whether text contains a value",
        category: "Text",
        version: crate::VERSION,
        inputs: &[
            TEXT_IN,
            port("value", Type::Text, "Text to look for"),
            opt("case_sensitive", Type::Boolean, "true", "Match letter case"),
        ],
        outputs: &[port("result", Type::Boolean, "True when found")],
        errors: &[],
        run: contains,
    },
    Operation {
        id: "std.text.length",
        name: "Length",
        description: "Number of characters (Unicode scalar values)",
        category: "Text",
        version: crate::VERSION,
        inputs: &[TEXT_IN],
        outputs: &[port("result", Type::Integer, "Character count")],
        errors: &[],
        run: length,
    },
    Operation {
        id: "std.text.normalize_unicode",
        name: "Normalize Unicode",
        description: "Unicode normalization (NFC by default) so equal-looking text compares equal",
        category: "Text",
        version: crate::VERSION,
        inputs: &[
            TEXT_IN,
            opt("form", Type::Text, "\"NFC\"", "NFC, NFD, NFKC or NFKD"),
        ],
        outputs: &[RESULT],
        errors: &["invalid_input"],
        run: normalize_unicode,
    },
    Operation {
        id: "std.text.normalize_line_endings",
        name: "Normalize line endings",
        description: "Turn CRLF and CR line endings into LF (or CRLF)",
        category: "Text",
        version: crate::VERSION,
        inputs: &[TEXT_IN, opt("style", Type::Text, "\"lf\"", "lf or crlf")],
        outputs: &[RESULT],
        errors: &["invalid_input"],
        run: normalize_line_endings,
    },
    Operation {
        id: "std.text.collapse_whitespace",
        name: "Collapse whitespace",
        description: "Replace every run of whitespace with a single space",
        category: "Text",
        version: crate::VERSION,
        inputs: &[TEXT_IN],
        outputs: &[RESULT],
        errors: &[],
        run: collapse_whitespace,
    },
];

#[cfg(test)]
mod tests {
    use crate::{code, res};
    use serde_json::json;

    #[test]
    fn case_and_trim() {
        assert_eq!(
            res("std.text.trim", json!({"text": " \t hi there \n"})),
            "hi there"
        );
        assert_eq!(
            res("std.text.lowercase", json!({"text": "ÄBC Straße"})),
            "äbc straße"
        );
        assert_eq!(
            res("std.text.uppercase", json!({"text": "straße"})),
            "STRASSE"
        );
    }

    #[test]
    fn split_and_join() {
        assert_eq!(
            res("std.text.split", json!({"text": "a, b,,c"})),
            json!(["a", " b", "", "c"])
        );
        assert_eq!(
            res(
                "std.text.split",
                json!({"text": "a, b,,c", "trim": true, "skip_empty": true})
            ),
            json!(["a", "b", "c"])
        );
        assert_eq!(res("std.text.split", json!({"text": ""})), json!([]));
        assert_eq!(
            code("std.text.split", json!({"text": "a", "delimiter": ""})),
            "invalid_input"
        );
        assert_eq!(
            res(
                "std.text.join",
                json!({"items": ["a", 1, true], "delimiter": "-"})
            ),
            "a-1-true"
        );
        assert_eq!(
            code("std.text.join", json!({"items": [[1]]})),
            "invalid_type"
        );
    }

    #[test]
    fn replace_contains_length() {
        assert_eq!(
            res(
                "std.text.replace",
                json!({"text": "a-b-c", "find": "-", "replacement": "+"})
            ),
            "a+b+c"
        );
        assert_eq!(
            res(
                "std.text.replace",
                json!({"text": "a-b-c", "find": "-", "all": false})
            ),
            "ab-c"
        );
        assert_eq!(
            code("std.text.replace", json!({"text": "a", "find": ""})),
            "invalid_input"
        );
        assert_eq!(
            res(
                "std.text.contains",
                json!({"text": "Hello", "value": "hell"})
            ),
            false
        );
        assert_eq!(
            res(
                "std.text.contains",
                json!({"text": "Hello", "value": "hell", "case_sensitive": false})
            ),
            true
        );
        assert_eq!(res("std.text.length", json!({"text": "héllo👋"})), 6);
    }

    #[test]
    fn normalization() {
        let decomposed = "e\u{301}";
        assert_eq!(
            res("std.text.normalize_unicode", json!({"text": decomposed})),
            "\u{e9}"
        );
        assert_eq!(
            res(
                "std.text.normalize_unicode",
                json!({"text": "ﬁ", "form": "NFKC"})
            ),
            "fi"
        );
        assert_eq!(
            code(
                "std.text.normalize_unicode",
                json!({"text": "a", "form": "X"})
            ),
            "invalid_input"
        );
        assert_eq!(
            res(
                "std.text.normalize_line_endings",
                json!({"text": "a\r\nb\rc\n"})
            ),
            "a\nb\nc\n"
        );
        assert_eq!(
            res(
                "std.text.normalize_line_endings",
                json!({"text": "a\nb", "style": "crlf"})
            ),
            "a\r\nb"
        );
        assert_eq!(
            res(
                "std.text.collapse_whitespace",
                json!({"text": " a \t\n b  c "})
            ),
            " a b c "
        );
    }
}
