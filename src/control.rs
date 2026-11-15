//! Errors as values on the canvas: raise a structured error on purpose
//! (for example on the invalid branch of a condition).
use crate::{Args, OpError, Operation, Outputs, Type, opt, port};

fn valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 64
        && code.starts_with(|c: char| c.is_ascii_lowercase())
        && code
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn raise(a: &Args) -> Result<Outputs, OpError> {
    let code = a.text("code");
    if !valid_code(code) {
        return Err(OpError::new(
            "invalid_input",
            "The error code must use a-z, 0-9 and _ (start with a letter, at most 64)",
        ));
    }
    Err(OpError::new(
        code,
        a.text("message").chars().take(500).collect::<String>(),
    ))
}

pub(crate) static OPERATIONS: &[Operation] = &[Operation {
    id: "std.error.raise",
    name: "Raise error",
    description: "Stop with a structured error (code and message). Error handling and failure branches apply",
    category: "Errors",
    version: crate::VERSION,
    inputs: &[
        opt(
            "code",
            Type::Text,
            "\"error\"",
            "Machine-readable code, e.g. invalid_total",
        ),
        port("message", Type::Text, "What went wrong, for people"),
    ],
    // Never produced: the node always fails. Declared so the contract
    // shows what a failure branch can read.
    outputs: &[],
    errors: &["<code>"],
    run: raise,
}];

#[cfg(test)]
mod tests {
    use crate::run;
    use serde_json::json;

    #[test]
    fn raise_fails_with_the_given_code() {
        let e = run(
            "std.error.raise",
            json!({"code": "invalid_total", "message": "Total must be nonzero"}),
        )
        .unwrap_err();
        assert_eq!(
            (e.code.as_str(), e.message.as_str()),
            ("invalid_total", "Total must be nonzero")
        );
        assert_eq!(
            run(
                "std.error.raise",
                json!({"code": "Bad Code", "message": "x"})
            )
            .unwrap_err()
            .code,
            "invalid_input"
        );
    }
}
