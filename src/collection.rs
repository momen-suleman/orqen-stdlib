//! List and object operations.
use serde_json::Value;

use crate::{Args, MAX_ITEMS, OpError, Operation, Outputs, Type, opt, port, result};

/// A list position: negative counts from the end (`-1` is the last item).
fn position(index: i64, len: usize) -> Option<usize> {
    let len = i64::try_from(len).ok()?;
    let i = if index < 0 {
        len.checked_add(index)?
    } else {
        index
    };
    (0..len).contains(&i).then_some(i as usize)
}

fn get(a: &Args) -> Result<Outputs, OpError> {
    let list = a.list("list");
    match position(a.integer("index"), list.len()) {
        Some(i) => result(list[i].clone()),
        None if a.has("default") => result(a.any("default").clone()),
        None => Err(OpError::new(
            "index_out_of_range",
            format!(
                "Index {} is outside a list of {}",
                a.integer("index"),
                list.len()
            ),
        )),
    }
}

fn count(a: &Args) -> Result<Outputs, OpError> {
    result(Value::from(a.list("list").len() as i64))
}

fn append(a: &Args) -> Result<Outputs, OpError> {
    let mut list = a.list("list").to_vec();
    if list.len() >= MAX_ITEMS {
        return Err(OpError::new(
            "too_large",
            format!("Lists hold at most {MAX_ITEMS} items"),
        ));
    }
    list.push(a.any("item").clone());
    result(Value::Array(list))
}

fn slice(a: &Args) -> Result<Outputs, OpError> {
    let list = a.list("list");
    let len = list.len() as i64;
    let clamp = |i: i64| -> usize {
        let i = if i < 0 { len.saturating_add(i) } else { i };
        i.clamp(0, len) as usize
    };
    let start = clamp(a.integer("start"));
    let end = if a.has("end") {
        clamp(a.integer("end"))
    } else {
        list.len()
    };
    result(Value::Array(if start < end {
        list[start..end].to_vec()
    } else {
        Vec::new()
    }))
}

fn object_get(a: &Args) -> Result<Outputs, OpError> {
    let key = a.text("key");
    let mut current = a.any("object");
    for segment in key.split('.') {
        let next = match current {
            Value::Object(map) => map.get(segment),
            Value::Array(items) => segment
                .parse::<i64>()
                .ok()
                .and_then(|i| position(i, items.len()))
                .map(|i| &items[i]),
            _ => None,
        };
        match next {
            Some(v) => current = v,
            None if a.has("default") => return result(a.any("default").clone()),
            None => {
                return Err(OpError::new("missing_key", format!("No value at '{key}'")));
            }
        }
    }
    result(current.clone())
}

fn object_keys(a: &Args) -> Result<Outputs, OpError> {
    let keys = a
        .object("object")
        .map(|m| m.keys().map(|k| Value::String(k.clone())).collect())
        .unwrap_or_default();
    result(Value::Array(keys))
}

fn object_set(a: &Args) -> Result<Outputs, OpError> {
    let mut map = a.object("object").cloned().unwrap_or_default();
    let key = a.text("key");
    if key.is_empty() {
        return Err(OpError::new("invalid_input", "The key cannot be empty"));
    }
    map.insert(key.to_owned(), a.any("value").clone());
    result(Value::Object(map))
}

const LIST: crate::Port = port("list", Type::List, "The list");
const DEFAULT: crate::Port = opt(
    "default",
    Type::Any,
    "null",
    "Returned instead of failing when there is no value",
);

pub(crate) static OPERATIONS: &[Operation] = &[
    Operation {
        id: "std.collection.get",
        name: "Get item",
        description: "The item at a position (0 is the first, -1 the last)",
        category: "Collections",
        version: crate::VERSION,
        inputs: &[LIST, port("index", Type::Integer, "Position"), DEFAULT],
        outputs: &[port("result", Type::Any, "The item")],
        errors: &["index_out_of_range"],
        run: get,
    },
    Operation {
        id: "std.collection.count",
        name: "Count",
        description: "Number of items in a list",
        category: "Collections",
        version: crate::VERSION,
        inputs: &[LIST],
        outputs: &[port("result", Type::Integer, "Item count")],
        errors: &[],
        run: count,
    },
    Operation {
        id: "std.collection.append",
        name: "Append",
        description: "A new list with an item added at the end",
        category: "Collections",
        version: crate::VERSION,
        inputs: &[LIST, port("item", Type::Any, "The item to add")],
        outputs: &[port("result", Type::List, "The new list")],
        errors: &[],
        run: append,
    },
    Operation {
        id: "std.collection.slice",
        name: "Slice",
        description: "Items from start (inclusive) to end (exclusive); negative positions count from the end",
        category: "Collections",
        version: crate::VERSION,
        inputs: &[
            LIST,
            opt("start", Type::Integer, "0", "First position"),
            opt(
                "end",
                Type::Integer,
                "null",
                "Stop before this position (empty: the end)",
            ),
        ],
        outputs: &[port("result", Type::List, "The items")],
        errors: &[],
        run: slice,
    },
    Operation {
        id: "std.object.get",
        name: "Get field",
        description: "A field of an object by key or dotted path (a.b.0)",
        category: "Objects",
        version: crate::VERSION,
        inputs: &[
            port("object", Type::Object, "The object"),
            port("key", Type::Text, "Key or dotted path"),
            DEFAULT,
        ],
        outputs: &[port("result", Type::Any, "The value")],
        errors: &["missing_key"],
        run: object_get,
    },
    Operation {
        id: "std.object.keys",
        name: "Keys",
        description: "The keys of an object",
        category: "Objects",
        version: crate::VERSION,
        inputs: &[port("object", Type::Object, "The object")],
        outputs: &[port("result", Type::List, "Keys, as text")],
        errors: &[],
        run: object_keys,
    },
    Operation {
        id: "std.object.set",
        name: "Set field",
        description: "A new object with one field set",
        category: "Objects",
        version: crate::VERSION,
        inputs: &[
            port("object", Type::Object, "The object"),
            port("key", Type::Text, "Key (not empty)"),
            port("value", Type::Any, "The value"),
        ],
        outputs: &[port("result", Type::Object, "The new object")],
        errors: &["invalid_input"],
        run: object_set,
    },
];

#[cfg(test)]
mod tests {
    use crate::{code, res};
    use serde_json::json;

    #[test]
    fn lists() {
        let l = json!(["a", "b", "c"]);
        assert_eq!(
            res("std.collection.get", json!({"list": l, "index": 0})),
            "a"
        );
        assert_eq!(
            res("std.collection.get", json!({"list": l, "index": -1})),
            "c"
        );
        assert_eq!(
            code("std.collection.get", json!({"list": l, "index": 3})),
            "index_out_of_range"
        );
        assert_eq!(
            res(
                "std.collection.get",
                json!({"list": l, "index": 9, "default": "z"})
            ),
            "z"
        );
        assert_eq!(res("std.collection.count", json!({"list": l})), 3);
        assert_eq!(
            res("std.collection.append", json!({"list": l, "item": 1})),
            json!(["a", "b", "c", 1])
        );
        assert_eq!(
            res("std.collection.slice", json!({"list": l, "start": 1})),
            json!(["b", "c"])
        );
        assert_eq!(
            res(
                "std.collection.slice",
                json!({"list": l, "start": -2, "end": -1})
            ),
            json!(["b"])
        );
        assert_eq!(
            res(
                "std.collection.slice",
                json!({"list": l, "start": 5, "end": 1})
            ),
            json!([])
        );
    }

    #[test]
    fn objects() {
        let o = json!({"a": {"b": [10, 20]}});
        assert_eq!(
            res("std.object.get", json!({"object": o, "key": "a.b.1"})),
            20
        );
        assert_eq!(
            code("std.object.get", json!({"object": o, "key": "x"})),
            "missing_key"
        );
        assert_eq!(res("std.object.keys", json!({"object": o})), json!(["a"]));
        assert_eq!(
            res(
                "std.object.set",
                json!({"object": {}, "key": "k", "value": [1]})
            ),
            json!({"k": [1]})
        );
    }
}
