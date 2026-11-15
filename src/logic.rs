//! Comparison and boolean operations.
use serde_json::Value;

use crate::number::Num;
use crate::{Args, OpError, Operation, Outputs, Type, port, result};

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

fn and(a: &Args) -> Result<Outputs, OpError> {
    result(Value::Bool(a.boolean("a") && a.boolean("b")))
}

fn or(a: &Args) -> Result<Outputs, OpError> {
    result(Value::Bool(a.boolean("a") || a.boolean("b")))
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
        description: "True when both are true",
        category: "Logic",
        version: crate::VERSION,
        inputs: &[BOOL_A, BOOL_B],
        outputs: &[OUT],
        errors: &[],
        run: and,
    },
    Operation {
        id: "std.logic.or",
        name: "Or",
        description: "True when either is true",
        category: "Logic",
        version: crate::VERSION,
        inputs: &[BOOL_A, BOOL_B],
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
];

#[cfg(test)]
mod tests {
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
        assert_eq!(res("std.logic.not", json!({"value": false})), true);
        assert_eq!(
            code("std.logic.not", json!({"value": "false"})),
            "invalid_type"
        );
    }
}
