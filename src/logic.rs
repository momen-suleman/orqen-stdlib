//! Comparison and boolean operations.
use serde_json::Value;

use crate::number::Num;
use crate::{Args, OpError, Operation, Outputs, Type, opt, port, result};

/// Structural equality where numbers compare by value (`1 == 1.0`).
pub(crate) fn equal_values(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => match (Num::from_json(a), Num::from_json(b)) {
            (Some(Num::Int(x)), Some(Num::Int(y))) => x == y,
            (Some(x), Some(y)) => x.as_f64() == y.as_f64(),
            _ => false,
        },
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| equal_values(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| equal_values(v, w)))
        }
        _ => a == b,
    }
}

fn compare(a: &Args) -> std::cmp::Ordering {
    match (a.number("a"), a.number("b")) {
        (Num::Int(x), Num::Int(y)) => x.cmp(&y),
        (x, y) => x
            .as_f64()
            .partial_cmp(&y.as_f64())
            .unwrap_or(std::cmp::Ordering::Equal),
    }
}

fn equal(a: &Args) -> Result<Outputs, OpError> {
    result(Value::Bool(equal_values(a.any("a"), a.any("b"))))
}

fn not_equal(a: &Args) -> Result<Outputs, OpError> {
    result(Value::Bool(!equal_values(a.any("a"), a.any("b"))))
}

fn greater_than(a: &Args) -> Result<Outputs, OpError> {
    result(Value::Bool(compare(a).is_gt()))
}

fn less_than(a: &Args) -> Result<Outputs, OpError> {
    result(Value::Bool(compare(a).is_lt()))
}

/// The condition inputs of And / Or: `a`, `b` and the optional `c`-`f`.
const CONDITIONS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

fn and(a: &Args) -> Result<Outputs, OpError> {
    result(Value::Bool(CONDITIONS.iter().all(|c| a.boolean(c))))
}

fn or(a: &Args) -> Result<Outputs, OpError> {
    result(Value::Bool(CONDITIONS.iter().any(|c| a.boolean(c))))
}

fn not(a: &Args) -> Result<Outputs, OpError> {
    result(Value::Bool(!a.boolean("value")))
}

const ANY_A: crate::Port = port("a", Type::Any, "First value");
const ANY_B: crate::Port = port("b", Type::Any, "Second value");
const NUM_A: crate::Port = port("a", Type::Number, "First number");
const NUM_B: crate::Port = port("b", Type::Number, "Second number");
const BOOL_A: crate::Port = port("a", Type::Boolean, "First condition");
const BOOL_B: crate::Port = port("b", Type::Boolean, "Second condition");
const OUT: crate::Port = port("result", Type::Boolean, "True or false");
/// And: extra conditions default to true, so leaving them empty changes nothing.
const AND_INPUTS: &[crate::Port] = &[
    BOOL_A,
    BOOL_B,
    opt("c", Type::Boolean, "true", "More conditions (optional)"),
    opt("d", Type::Boolean, "true", "Optional"),
    opt("e", Type::Boolean, "true", "Optional"),
    opt("f", Type::Boolean, "true", "Optional"),
];
/// Or: extra conditions default to false, so leaving them empty changes nothing.
const OR_INPUTS: &[crate::Port] = &[
    BOOL_A,
    BOOL_B,
    opt("c", Type::Boolean, "false", "More conditions (optional)"),
    opt("d", Type::Boolean, "false", "Optional"),
    opt("e", Type::Boolean, "false", "Optional"),
    opt("f", Type::Boolean, "false", "Optional"),
];

/// One of two values, picked by a condition.
fn choose(a: &Args) -> Result<Outputs, OpError> {
    let picked = if a.boolean("condition") {
        "then"
    } else {
        "else"
    };
    result(a.any(picked).clone())
}

pub(crate) static OPERATIONS: &[Operation] = &[
    Operation {
        id: "std.logic.equal",
        name: "Equal",
        description: "Whether two values are equal (numbers by value, lists and objects deeply)",
        category: "Logic",
        version: crate::VERSION,
        inputs: &[ANY_A, ANY_B],
        outputs: &[OUT],
        errors: &[],
        run: equal,
    },
    Operation {
        id: "std.logic.not_equal",
        name: "Not equal",
        description: "Whether two values differ (the negation of Equal)",
        category: "Logic",
        version: crate::VERSION,
        inputs: &[ANY_A, ANY_B],
        outputs: &[OUT],
        errors: &[],
        run: not_equal,
    },
    Operation {
        id: "std.logic.greater_than",
        name: "Greater than",
        description: "a > b",
        category: "Logic",
        version: crate::VERSION,
        inputs: &[NUM_A, NUM_B],
        outputs: &[OUT],
        errors: &[],
        run: greater_than,
    },
    Operation {
        id: "std.logic.less_than",
        name: "Less than",
        description: "a < b",
        category: "Logic",
        version: crate::VERSION,
        inputs: &[NUM_A, NUM_B],
        outputs: &[OUT],
        errors: &[],
        run: less_than,
    },
    Operation {
        id: "std.logic.and",
        name: "And",
        description: "True when all conditions are true (a, b and up to four more)",
        category: "Logic",
        version: crate::VERSION,
        inputs: AND_INPUTS,
        outputs: &[OUT],
        errors: &[],
        run: and,
    },
    Operation {
        id: "std.logic.or",
        name: "Or",
        description: "True when any condition is true (a, b and up to four more)",
        category: "Logic",
        version: crate::VERSION,
        inputs: OR_INPUTS,
        outputs: &[OUT],
        errors: &[],
        run: or,
    },
    Operation {
        id: "std.logic.not",
        name: "Not",
        description: "The opposite of a condition",
        category: "Logic",
        version: crate::VERSION,
        inputs: &[port("value", Type::Boolean, "The condition")],
        outputs: &[OUT],
        errors: &[],
        run: not,
    },
    Operation {
        id: "std.logic.choose",
        name: "Choose",
        description: "The then value when the condition is true, otherwise the else value",
        category: "Logic",
        version: crate::VERSION,
        inputs: &[
            port("condition", Type::Boolean, "The condition"),
            port("then", Type::Any, "Value when true"),
            port("else", Type::Any, "Value when false"),
        ],
        outputs: &[port("result", Type::Any, "The chosen value")],
        errors: &[],
        run: choose,
    },
];

#[cfg(test)]
mod tests {
    #[test]
    fn choosing() {
        use serde_json::json;
        let pick = |c: bool| {
            crate::res(
                "std.logic.choose",
                json!({"condition": c, "then": "accepted", "else": ["x"]}),
            )
        };
        assert_eq!(pick(true), "accepted");
        assert_eq!(pick(false), json!(["x"]));
    }

    use crate::{code, res};
    use serde_json::json;

    #[test]
    fn comparisons() {
        assert_eq!(res("std.logic.equal", json!({"a": 1, "b": 1.0})), true);
        assert_eq!(res("std.logic.equal", json!({"a": "1", "b": 1})), false);
        assert_eq!(
            res(
                "std.logic.equal",
                json!({"a": {"x": [1, 2]}, "b": {"x": [1, 2.0]}})
            ),
            true
        );
        assert_eq!(res("std.logic.equal", json!({"a": null, "b": null})), true);
        assert_eq!(res("std.logic.not_equal", json!({"a": 200, "b": 0})), true);
        assert_eq!(
            res("std.logic.greater_than", json!({"a": 2.5, "b": 2})),
            true
        );
        assert_eq!(
            res(
                "std.logic.less_than",
                json!({"a": i64::MAX - 1, "b": i64::MAX})
            ),
            true
        );
        assert_eq!(
            code("std.logic.greater_than", json!({"a": "3", "b": 2})),
            "invalid_type"
        );
    }

    #[test]
    fn booleans() {
        assert_eq!(res("std.logic.and", json!({"a": true, "b": false})), false);
        assert_eq!(res("std.logic.or", json!({"a": true, "b": false})), true);
        // Up to six conditions; the extra ones are optional and neutral.
        assert_eq!(res("std.logic.or", json!({"a": false, "b": false})), false);
        assert_eq!(
            res("std.logic.or", json!({"a": false, "b": false, "c": true})),
            true
        );
        assert_eq!(
            res("std.logic.or", json!({"a": false, "b": false, "f": true})),
            true
        );
        assert_eq!(res("std.logic.and", json!({"a": true, "b": true})), true);
        assert_eq!(
            res("std.logic.and", json!({"a": true, "b": true, "d": false})),
            false
        );
        assert_eq!(code("std.logic.or", json!({"a": true})), "missing_input");
        assert_eq!(res("std.logic.not", json!({"value": false})), true);
        assert_eq!(
            code("std.logic.not", json!({"value": "false"})),
            "invalid_type"
        );
    }
}
