//! Number operations. See the numeric model in the crate documentation.
use serde_json::Value;

use crate::{Args, OpError, Operation, Outputs, Type, json_number, opt, port, result};

/// A number at run time: an exact `i64` or a finite `f64`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Num {
    Int(i64),
    Dec(f64),
}

impl Num {
    pub(crate) fn from_json(value: &Value) -> Option<Num> {
        let n = value.as_number()?;
        match n.as_i64() {
            Some(i) => Some(Num::Int(i)),
            None => n.as_f64().filter(|f| f.is_finite()).map(Num::Dec),
        }
    }

    pub(crate) fn as_f64(self) -> f64 {
        match self {
            Num::Int(i) => i as f64,
            Num::Dec(f) => f,
        }
    }

    fn to_json(self) -> Result<Value, OpError> {
        match self {
            Num::Int(i) => Ok(Value::from(i)),
            Num::Dec(f) => json_number(f).ok_or_else(overflow),
        }
    }
}

fn overflow() -> OpError {
    OpError::new("overflow", "The result is out of range")
}

fn arith(
    a: &Args,
    int: fn(i64, i64) -> Option<i64>,
    dec: fn(f64, f64) -> f64,
) -> Result<Outputs, OpError> {
    let value = match (a.number("a"), a.number("b")) {
        (Num::Int(x), Num::Int(y)) => Num::Int(int(x, y).ok_or_else(overflow)?),
        (x, y) => Num::Dec(dec(x.as_f64(), y.as_f64())),
    };
    result(value.to_json()?)
}

fn add(a: &Args) -> Result<Outputs, OpError> {
    arith(a, i64::checked_add, |x, y| x + y)
}

fn subtract(a: &Args) -> Result<Outputs, OpError> {
    arith(a, i64::checked_sub, |x, y| x - y)
}

fn multiply(a: &Args) -> Result<Outputs, OpError> {
    arith(a, i64::checked_mul, |x, y| x * y)
}

fn divide(a: &Args) -> Result<Outputs, OpError> {
    let (x, y) = (a.number("a"), a.number("b"));
    if y.as_f64() == 0.0 {
        return Err(OpError::new("division_by_zero", "The divisor is zero"));
    }
    let value = match (x, y) {
        (Num::Int(x), Num::Int(y)) => match (x.checked_rem(y), x.checked_div(y)) {
            (Some(0), Some(q)) => Num::Int(q),
            (Some(_), Some(_)) => Num::Dec(x as f64 / y as f64),
            _ => return Err(overflow()),
        },
        (x, y) => Num::Dec(x.as_f64() / y.as_f64()),
    };
    result(value.to_json()?)
}

pub(crate) const MAX_PRECISION: i64 = 15;

/// Round the shortest decimal representation of `value` to `precision`
/// fraction digits, half away from zero.
pub(crate) fn round_decimal(value: f64, precision: usize) -> f64 {
    let repr = format!("{value}");
    let (negative, digits) = match repr.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, repr.as_str()),
    };
    let (int_part, frac_part) = digits.split_once('.').unwrap_or((digits, ""));
    if frac_part.len() <= precision {
        return value;
    }
    let mut kept: Vec<u8> = int_part
        .bytes()
        .chain(frac_part.bytes().take(precision))
        .collect();
    if frac_part.as_bytes()[precision] >= b'5' {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    let split = kept.len() - precision;
    let mut text = String::from(if negative { "-" } else { "" });
    text.push_str(std::str::from_utf8(&kept[..split]).unwrap_or("0"));
    if precision > 0 {
        text.push('.');
        text.push_str(std::str::from_utf8(&kept[split..]).unwrap_or("0"));
    }
    text.parse().unwrap_or(value)
}

fn round(a: &Args) -> Result<Outputs, OpError> {
    let precision = a.integer("precision");
    if !(0..=MAX_PRECISION).contains(&precision) {
        return Err(OpError::new(
            "invalid_input",
            format!("Precision must be 0 to {MAX_PRECISION}"),
        ));
    }
    let value = match a.number("value") {
        Num::Int(i) => Num::Int(i),
        Num::Dec(f) => Num::Dec(round_decimal(f, precision as usize)),
    };
    result(value.to_json()?)
}

/// Sum of a list of numbers: exact while every value is an integer.
fn sum(a: &Args) -> Result<Outputs, OpError> {
    let mut total = Num::Int(0);
    for (i, item) in a.list("values").iter().enumerate() {
        let n = Num::from_json(item).ok_or_else(|| {
            OpError::new(
                "invalid_type",
                format!(
                    "Item {i} of 'values' is not a number: {}",
                    crate::truncate_json(item)
                ),
            )
        })?;
        total = match (total, n) {
            (Num::Int(x), Num::Int(y)) => Num::Int(x.checked_add(y).ok_or_else(overflow)?),
            (x, y) => {
                let f = x.as_f64() + y.as_f64();
                if !f.is_finite() {
                    return Err(overflow());
                }
                Num::Dec(f)
            }
        };
    }
    result(total.to_json()?)
}

const A: crate::Port = port("a", Type::Number, "First operand");
const B: crate::Port = port("b", Type::Number, "Second operand");
const OUT: crate::Port = port("result", Type::Number, "The result");

pub(crate) static OPERATIONS: &[Operation] = &[
    Operation {
        id: "std.number.sum",
        name: "Sum",
        description: "Add up a list of numbers (exact for integers; an empty list is 0)",
        category: "Numbers",
        version: crate::VERSION,
        inputs: &[port("values", Type::List, "Numbers to add up")],
        outputs: &[OUT],
        errors: &["overflow"],
        run: sum,
    },
    Operation {
        id: "std.number.add",
        name: "Add",
        description: "a + b (exact for integers; fails on overflow)",
        category: "Numbers",
        version: crate::VERSION,
        inputs: &[A, B],
        outputs: &[OUT],
        errors: &["overflow"],
        run: add,
    },
    Operation {
        id: "std.number.subtract",
        name: "Subtract",
        description: "a − b (exact for integers; fails on overflow)",
        category: "Numbers",
        version: crate::VERSION,
        inputs: &[A, B],
        outputs: &[OUT],
        errors: &["overflow"],
        run: subtract,
    },
    Operation {
        id: "std.number.multiply",
        name: "Multiply",
        description: "a × b (exact for integers; fails on overflow)",
        category: "Numbers",
        version: crate::VERSION,
        inputs: &[A, B],
        outputs: &[OUT],
        errors: &["overflow"],
        run: multiply,
    },
    Operation {
        id: "std.number.divide",
        name: "Divide",
        description: "a ÷ b; fails when b is zero",
        category: "Numbers",
        version: crate::VERSION,
        inputs: &[
            port("a", Type::Number, "Dividend"),
            port("b", Type::Number, "Divisor (not zero)"),
        ],
        outputs: &[OUT],
        errors: &["division_by_zero", "overflow"],
        run: divide,
    },
    Operation {
        id: "std.number.round",
        name: "Round",
        description: "Round to a number of decimal places, half away from zero",
        category: "Numbers",
        version: crate::VERSION,
        inputs: &[
            port("value", Type::Number, "The number"),
            opt("precision", Type::Integer, "0", "Decimal places, 0-15"),
        ],
        outputs: &[OUT],
        errors: &["invalid_input"],
        run: round,
    },
];

#[cfg(test)]
mod tests {
    use crate::{code, res};
    use serde_json::json;

    #[test]
    fn integer_arithmetic_is_exact_and_checked() {
        assert_eq!(res("std.number.add", json!({"a": 2, "b": 3})), 5);
        assert_eq!(res("std.number.subtract", json!({"a": 2, "b": 3})), -1);
        assert_eq!(res("std.number.multiply", json!({"a": 4, "b": 25})), 100);
        assert_eq!(
            code("std.number.add", json!({"a": i64::MAX, "b": 1})),
            "overflow"
        );
        assert_eq!(
            code("std.number.multiply", json!({"a": i64::MAX, "b": 2})),
            "overflow"
        );
    }

    #[test]
    fn decimals() {
        assert_eq!(res("std.number.add", json!({"a": 0.5, "b": 0.25})), 0.75);
        assert_eq!(
            res("std.number.multiply", json!({"a": 0.125, "b": 100})),
            12.5
        );
        assert_eq!(res("std.number.multiply", json!({"a": 2.5, "b": 2})), 5);
        assert_eq!(
            code("std.number.multiply", json!({"a": 1e308, "b": 10})),
            "overflow"
        );
    }

    #[test]
    fn division() {
        assert_eq!(res("std.number.divide", json!({"a": 10, "b": 2})), 5);
        assert_eq!(res("std.number.divide", json!({"a": 25, "b": 200})), 0.125);
        assert_eq!(
            code("std.number.divide", json!({"a": 1, "b": 0})),
            "division_by_zero"
        );
        assert_eq!(
            code("std.number.divide", json!({"a": 1, "b": 0.0})),
            "division_by_zero"
        );
        assert_eq!(
            code("std.number.divide", json!({"a": i64::MIN, "b": -1})),
            "overflow"
        );
    }

    #[test]
    fn sums() {
        assert_eq!(res("std.number.sum", json!({"values": [5, 3, 2]})), 10);
        assert_eq!(res("std.number.sum", json!({"values": []})), 0);
        assert_eq!(res("std.number.sum", json!({"values": [0.5, 2]})), 2.5);
        assert_eq!(res("std.number.sum", json!({"values": [0.25, 0.75]})), 1);
        assert_eq!(
            code("std.number.sum", json!({"values": [1, "2"]})),
            "invalid_type"
        );
        assert_eq!(
            code("std.number.sum", json!({"values": [i64::MAX, 1]})),
            "overflow"
        );
        assert_eq!(
            code("std.number.sum", json!({"values": "1,2"})),
            "invalid_type"
        );
    }

    #[test]
    fn rounding() {
        assert_eq!(
            res("std.number.round", json!({"value": 1.005, "precision": 2})),
            1.01
        );
        assert_eq!(res("std.number.round", json!({"value": 2.5})), 3);
        assert_eq!(res("std.number.round", json!({"value": -2.5})), -3);
        assert_eq!(
            res("std.number.round", json!({"value": 9.995, "precision": 2})),
            10
        );
        assert_eq!(
            res(
                "std.number.round",
                json!({"value": 33.333333, "precision": 2})
            ),
            33.33
        );
        assert_eq!(
            res("std.number.round", json!({"value": 7, "precision": 2})),
            7
        );
        assert_eq!(
            code("std.number.round", json!({"value": 1, "precision": 16})),
            "invalid_input"
        );
    }
}
