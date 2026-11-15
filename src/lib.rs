//! # Orqen standard library
//!
//! Small, deterministic, typed operations that Orqen workflows are built
//! from: text, numbers, logic, collections, objects, type conversion and
//! errors. Each operation is implemented **once** here and reused by every
//! workflow (email, CRM, search, documents ...) instead of being rewritten
//! inside each domain package.
//!
//! The crate has no dependency on Orqen. It exposes:
//!
//! * [`operations`] — the catalog: id, name, category, typed input and
//!   output [`Port`]s (with defaults), error codes;
//! * [`call`] — run one operation on a JSON object of inputs. Inputs are
//!   validated against the contract (unknown, missing and mistyped inputs
//!   are refused), defaults are applied, and outputs are checked against
//!   the declared types before they are returned;
//! * [`Type::accepts`] — the connection rule the Canvas and the engine use
//!   to decide whether an output may feed an input;
//! * [`workflows`] — the published micro-workflows of this package, as the
//!   Library JSON that Orqen imports.
//!
//! Every operation is pure: no I/O, no clock, no randomness, no
//! permissions. Same input, same output.
//!
//! ## Numeric model
//!
//! `number` values are JSON numbers. Two kinds exist at run time:
//!
//! * **integers** — signed 64-bit. `add`, `subtract` and `multiply` of two
//!   integers are exact; a result outside `i64` fails with `overflow`
//!   instead of wrapping;
//! * **decimals** — IEEE-754 binary64. A result that is not finite fails
//!   with `overflow`; `NaN` and infinities can never be produced or read.
//!
//! `divide` of two integers returns an integer when the division is exact,
//! otherwise a decimal; a zero divisor fails with `division_by_zero`.
//! A decimal whose value is integral (and within ±2^53) is written as an
//! integer. `round` rounds the shortest decimal representation of the value
//! half away from zero, so `round(1.005, 2)` is `1.01`, as written.
//!
//! Text is never silently turned into a number: use `std.type.to_number`.
use serde_json::{Map, Number, Value};

mod collection;
mod control;
mod logic;
mod number;
mod text;
mod types;
pub mod workflows;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PACKAGE: &str = "orqen-stdlib";

/// Largest text input or output an operation accepts (bytes).
pub const MAX_TEXT: usize = 1 << 20;
/// Largest list an operation accepts or produces.
pub const MAX_ITEMS: usize = 100_000;

/// Port types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Text,
    Integer,
    /// Integer or decimal.
    Number,
    Boolean,
    List,
    Object,
    /// Any JSON value, including null.
    Any,
}

impl Type {
    pub const ALL: [Type; 7] = [
        Type::Text,
        Type::Integer,
        Type::Number,
        Type::Boolean,
        Type::List,
        Type::Object,
        Type::Any,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Type::Text => "text",
            Type::Integer => "integer",
            Type::Number => "number",
            Type::Boolean => "boolean",
            Type::List => "list",
            Type::Object => "object",
            Type::Any => "any",
        }
    }

    pub fn parse(name: &str) -> Option<Type> {
        Type::ALL.into_iter().find(|t| t.name() == name)
    }

    /// Can an output of type `from` feed an input of this type?
    ///
    /// Exact matches connect; `integer` feeds `number`; `any` feeds and
    /// accepts everything (the value is then checked at run time). Nothing
    /// else connects — in particular text never feeds a number: convert it
    /// with `std.type.to_number`.
    pub fn accepts(self, from: Type) -> bool {
        self == from
            || self == Type::Any
            || from == Type::Any
            || (self == Type::Number && from == Type::Integer)
    }

    /// Does `value` have this type?
    pub fn check(self, value: &Value) -> bool {
        match self {
            Type::Text => value.is_string(),
            Type::Integer => value.as_i64().is_some(),
            Type::Number => value.is_number(),
            Type::Boolean => value.is_boolean(),
            Type::List => value.is_array(),
            Type::Object => value.is_object(),
            Type::Any => true,
        }
    }
}

/// One typed input or output.
#[derive(Debug, Clone, Copy)]
pub struct Port {
    pub name: &'static str,
    pub ty: Type,
    /// JSON literal of the default value. An input with a default is
    /// optional; one without is required.
    pub default: Option<&'static str>,
    pub description: &'static str,
}

impl Port {
    pub fn required(&self) -> bool {
        self.default.is_none()
    }

    pub fn default_value(&self) -> Option<Value> {
        self.default
            .map(|d| serde_json::from_str(d).expect("valid default literal"))
    }
}

const fn port(name: &'static str, ty: Type, description: &'static str) -> Port {
    Port {
        name,
        ty,
        default: None,
        description,
    }
}

const fn opt(
    name: &'static str,
    ty: Type,
    default: &'static str,
    description: &'static str,
) -> Port {
    Port {
        name,
        ty,
        default: Some(default),
        description,
    }
}

/// A failed operation: a stable machine-readable code and a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpError {
    pub code: String,
    pub message: String,
}

impl OpError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for OpError {}

pub type Outputs = Map<String, Value>;
type Run = fn(&Args) -> Result<Outputs, OpError>;

/// One atomic operation and its contract.
#[derive(Debug, Clone, Copy)]
pub struct Operation {
    /// Stable identifier, `std.<area>.<verb>`. A breaking change ships
    /// under a new id; the id never changes meaning.
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// Palette group.
    pub category: &'static str,
    pub version: &'static str,
    pub inputs: &'static [Port],
    pub outputs: &'static [Port],
    /// Error codes the operation can fail with (besides the input checks
    /// every operation shares: `missing_input`, `invalid_type`,
    /// `unknown_input`, `too_large`).
    pub errors: &'static [&'static str],
    run: Run,
}

impl Operation {
    pub fn input(&self, name: &str) -> Option<&'static Port> {
        self.inputs.iter().find(|p| p.name == name)
    }

    pub fn output(&self, name: &str) -> Option<&'static Port> {
        self.outputs.iter().find(|p| p.name == name)
    }
}

/// Error codes shared by every operation.
pub const COMMON_ERRORS: [&str; 4] = [
    "missing_input",
    "invalid_type",
    "unknown_input",
    "too_large",
];

/// Categories in palette order.
pub const CATEGORIES: [&str; 7] = [
    "Text",
    "Numbers",
    "Logic",
    "Collections",
    "Objects",
    "Types",
    "Errors",
];

/// Validated inputs of one call.
pub struct Args {
    values: Map<String, Value>,
}

impl Args {
    fn get(&self, name: &str) -> &Value {
        self.values.get(name).unwrap_or(&Value::Null)
    }

    pub(crate) fn has(&self, name: &str) -> bool {
        self.values.get(name).is_some_and(|v| !v.is_null())
    }

    pub(crate) fn text(&self, name: &str) -> &str {
        self.get(name).as_str().unwrap_or("")
    }

    pub(crate) fn integer(&self, name: &str) -> i64 {
        self.get(name).as_i64().unwrap_or(0)
    }

    pub(crate) fn boolean(&self, name: &str) -> bool {
        self.get(name).as_bool().unwrap_or(false)
    }

    pub(crate) fn number(&self, name: &str) -> number::Num {
        number::Num::from_json(self.get(name)).unwrap_or(number::Num::Int(0))
    }

    pub(crate) fn list(&self, name: &str) -> &[Value] {
        self.get(name).as_array().map(Vec::as_slice).unwrap_or(&[])
    }

    pub(crate) fn object(&self, name: &str) -> Option<&Map<String, Value>> {
        self.get(name).as_object()
    }

    pub(crate) fn any(&self, name: &str) -> &Value {
        self.get(name)
    }
}

pub(crate) fn one(name: &str, value: Value) -> Outputs {
    let mut out = Map::new();
    out.insert(name.to_owned(), value);
    out
}

pub(crate) fn result(value: Value) -> Result<Outputs, OpError> {
    Ok(one("result", value))
}

pub(crate) fn text_result(text: String) -> Result<Outputs, OpError> {
    if text.len() > MAX_TEXT {
        return Err(OpError::new(
            "too_large",
            format!("The result is longer than {MAX_TEXT} bytes"),
        ));
    }
    result(Value::String(text))
}

static OPERATIONS: std::sync::LazyLock<Vec<Operation>> = std::sync::LazyLock::new(|| {
    let mut all = Vec::new();
    all.extend_from_slice(text::OPERATIONS);
    all.extend_from_slice(number::OPERATIONS);
    all.extend_from_slice(logic::OPERATIONS);
    all.extend_from_slice(collection::OPERATIONS);
    all.extend_from_slice(types::OPERATIONS);
    all.extend_from_slice(control::OPERATIONS);
    all
});

/// Every operation, grouped by category.
pub fn operations() -> &'static [Operation] {
    &OPERATIONS
}

pub fn find(id: &str) -> Option<&'static Operation> {
    operations().iter().find(|o| o.id == id)
}

/// Is `id` a standard library operation id (`std.*`)?
pub fn is_std_id(id: &str) -> bool {
    id.starts_with("std.")
}

fn size_ok(value: &Value) -> bool {
    match value {
        Value::String(s) => s.len() <= MAX_TEXT,
        Value::Array(items) => items.len() <= MAX_ITEMS && items.iter().all(size_ok),
        Value::Object(map) => map.len() <= MAX_ITEMS && map.values().all(size_ok),
        _ => true,
    }
}

fn describe(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "text",
        Value::Array(_) => "list",
        Value::Object(_) => "object",
    }
}

/// Run operation `id` with `inputs`.
///
/// Inputs are checked against the contract before the operation runs:
/// an unknown input, a missing required input or a value of the wrong type
/// fails with `unknown_input`, `missing_input` or `invalid_type`. Optional
/// inputs that are absent (or null) take their default. The outputs are
/// checked against their declared types too, so a contract can never be
/// silently broken by its implementation.
pub fn call(id: &str, inputs: &Map<String, Value>) -> Result<Outputs, OpError> {
    let op = find(id)
        .ok_or_else(|| OpError::new("unknown_operation", format!("Unknown operation '{id}'")))?;
    for name in inputs.keys() {
        if op.input(name).is_none() {
            return Err(OpError::new(
                "unknown_input",
                format!("{} has no input '{name}'", op.id),
            ));
        }
    }
    let mut values = Map::new();
    for port in op.inputs {
        let value = match inputs.get(port.name) {
            Some(v) if !v.is_null() || port.ty == Type::Any && port.required() => v.clone(),
            _ => match port.default_value() {
                Some(d) => d,
                None => {
                    return Err(OpError::new(
                        "missing_input",
                        format!("{}: input '{}' is required", op.id, port.name),
                    ));
                }
            },
        };
        if !port.ty.check(&value) && !(value.is_null() && port.default == Some("null")) {
            return Err(OpError::new(
                "invalid_type",
                format!(
                    "{}: input '{}' must be {}, got {}",
                    op.id,
                    port.name,
                    port.ty.name(),
                    describe(&value)
                ),
            ));
        }
        if !size_ok(&value) {
            return Err(OpError::new(
                "too_large",
                format!("{}: input '{}' is too large", op.id, port.name),
            ));
        }
        values.insert(port.name.to_owned(), value);
    }
    let outputs = (op.run)(&Args { values })?;
    for port in op.outputs {
        match outputs.get(port.name) {
            Some(v) if port.ty.check(v) => {}
            other => {
                return Err(OpError::new(
                    "contract_violation",
                    format!(
                        "{}: output '{}' must be {}, got {}",
                        op.id,
                        port.name,
                        port.ty.name(),
                        other.map_or("nothing", describe)
                    ),
                ));
            }
        }
    }
    Ok(outputs)
}

/// A JSON value shortened for an error message.
pub(crate) fn truncate_json(value: &Value) -> String {
    let text = value.to_string();
    match text.char_indices().nth(40) {
        Some((i, _)) => format!("{}…", &text[..i]),
        None => text,
    }
}

/// The JSON number for an `f64`, written as an integer when it is integral
/// and exactly representable; `None` when not finite.
pub(crate) fn json_number(value: f64) -> Option<Value> {
    if !value.is_finite() {
        return None;
    }
    if value.fract() == 0.0 && value.abs() <= 9_007_199_254_740_992.0 {
        return Some(Value::from(value as i64));
    }
    Number::from_f64(value).map(Value::Number)
}

#[cfg(test)]
pub(crate) fn run(id: &str, inputs: Value) -> Result<Outputs, OpError> {
    call(id, inputs.as_object().expect("object"))
}

#[cfg(test)]
pub(crate) fn res(id: &str, inputs: Value) -> Value {
    run(id, inputs).unwrap_or_else(|e| panic!("{id}: {e}"))["result"].clone()
}

#[cfg(test)]
pub(crate) fn code(id: &str, inputs: Value) -> String {
    run(id, inputs).expect_err("should fail").code
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    #[test]
    fn catalog_is_well_formed() {
        let mut ids = HashSet::new();
        for op in operations() {
            assert!(ids.insert(op.id), "duplicate id {}", op.id);
            assert!(
                op.id.starts_with("std.") && op.id.split('.').count() == 3,
                "{}",
                op.id
            );
            assert!(
                CATEGORIES.contains(&op.category),
                "{}: {}",
                op.id,
                op.category
            );
            assert!(!op.description.is_empty(), "{}", op.id);
            assert!(
                !op.outputs.is_empty() || op.category == "Errors",
                "{}",
                op.id
            );
            let mut names = HashSet::new();
            for p in op.inputs.iter().chain(op.outputs) {
                assert!(
                    names.insert((p.name, op.input(p.name).is_some())),
                    "{}: {}",
                    op.id,
                    p.name
                );
            }
            for p in op.inputs {
                if let Some(d) = p.default_value() {
                    assert!(
                        p.ty.check(&d) || d.is_null(),
                        "{}.{} default",
                        op.id,
                        p.name
                    );
                }
            }
        }
        assert!(operations().len() >= 30);
    }

    #[test]
    fn inputs_are_validated() {
        assert_eq!(code("std.text.trim", json!({})), "missing_input");
        assert_eq!(code("std.text.trim", json!({"text": 5})), "invalid_type");
        assert_eq!(
            code("std.text.trim", json!({"text": "a", "x": 1})),
            "unknown_input"
        );
        assert_eq!(code("std.nope.nope", json!({})), "unknown_operation");
        assert_eq!(
            code("std.number.add", json!({"a": "1", "b": 2})),
            "invalid_type"
        );
    }

    #[test]
    fn connection_rules() {
        assert!(Type::Number.accepts(Type::Integer));
        assert!(!Type::Integer.accepts(Type::Number));
        assert!(!Type::Number.accepts(Type::Text));
        assert!(!Type::Text.accepts(Type::Number));
        assert!(Type::Any.accepts(Type::List) && Type::Object.accepts(Type::Any));
        assert!(Type::List.accepts(Type::List));
    }
}
