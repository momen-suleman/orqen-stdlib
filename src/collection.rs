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

/// The largest number and its position; the first one wins a tie, so the
/// result is deterministic. An empty list gives `default` at index -1 when
/// a default is set, and fails otherwise.
fn max(a: &Args) -> Result<Outputs, OpError> {
    let mut best: Option<(usize, crate::number::Num)> = None;
    for (i, item) in a.list("values").iter().enumerate() {
        let n = crate::number::Num::from_json(item).ok_or_else(|| {
            OpError::new(
                "invalid_type",
                format!(
                    "Item {i} of 'values' is not a number: {}",
                    crate::truncate_json(item)
                ),
            )
        })?;
        let larger = match (&best, n) {
            (None, _) => true,
            (Some((_, crate::number::Num::Int(b))), crate::number::Num::Int(x)) => x > *b,
            (Some((_, b)), x) => x.as_f64() > b.as_f64(),
        };
        if larger {
            best = Some((i, n));
        }
    }
    let (index, value) = match best {
        Some((i, n)) => (i as i64, n),
        None if a.has("default") => (-1, a.number("default")),
        None => return Err(OpError::new("empty_list", "The list has no numbers")),
    };
    let mut out = crate::one("index", Value::from(index));
    out.insert(
        "result".to_owned(),
        match value {
            crate::number::Num::Int(i) => Value::from(i),
            crate::number::Num::Dec(f) => crate::json_number(f).unwrap_or(Value::Null),
        },
    );
    Ok(out)
}

/// Numbers or texts in order; a stable sort, so equal items keep their order.
fn sort(a: &Args) -> Result<Outputs, OpError> {
    let list = a.list("list");
    let mut items: Vec<Value> = list.to_vec();
    if items
        .iter()
        .all(|v| crate::number::Num::from_json(v).is_some())
    {
        items.sort_by(|x, y| {
            let (x, y) = (
                crate::number::Num::from_json(x).map_or(0.0, |n| n.as_f64()),
                crate::number::Num::from_json(y).map_or(0.0, |n| n.as_f64()),
            );
            x.total_cmp(&y)
        });
    } else if items.iter().all(Value::is_string) {
        items.sort_by(|x, y| x.as_str().cmp(&y.as_str()));
    } else {
        return Err(OpError::new(
            "invalid_type",
            "Sort needs a list of only numbers or only texts",
        ));
    }
    if a.boolean("descending") {
        // Reverse keeps a tie's later item first; restore input order for ties.
        items.reverse();
        let key = |v: &Value| v.clone();
        let mut i = 0;
        while i < items.len() {
            let mut j = i + 1;
            while j < items.len() && key(&items[j]) == key(&items[i]) {
                j += 1;
            }
            items[i..j].reverse();
            i = j;
        }
    }
    result(Value::Array(items))
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

/// Whether the list holds the item (numbers by value, lists and objects deeply).
fn contains(a: &Args) -> Result<Outputs, OpError> {
    let item = a.any("item");
    result(Value::Bool(
        a.list("list")
            .iter()
            .any(|v| crate::logic::equal_values(v, item)),
    ))
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

fn object_values(a: &Args) -> Result<Outputs, OpError> {
    let values = a
        .object("object")
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();
    result(Value::Array(values))
}

/// An object from a list of keys and a list of values at the same positions.
fn object_from_lists(a: &Args) -> Result<Outputs, OpError> {
    let (keys, values) = (a.list("keys"), a.list("values"));
    if keys.len() != values.len() {
        return Err(OpError::new(
            "invalid_input",
            format!(
                "{} keys but {} values; the lists must be the same length",
                keys.len(),
                values.len()
            ),
        ));
    }
    let mut map = serde_json::Map::new();
    for (i, (key, value)) in keys.iter().zip(values).enumerate() {
        let Some(key) = key.as_str().filter(|k| !k.is_empty()) else {
            return Err(OpError::new(
                "invalid_input",
                format!(
                    "Key {i} is not a non-empty text: {}",
                    crate::truncate_json(key)
                ),
            ));
        };
        if map.insert(key.to_owned(), value.clone()).is_some() {
            return Err(OpError::new(
                "invalid_input",
                format!("Key '{key}' appears twice"),
            ));
        }
    }
    result(Value::Object(map))
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
        id: "std.collection.max",
        name: "Largest",
        description: "The largest number in a list and its position (the first one on a tie)",
        category: "Collections",
        version: crate::VERSION,
        inputs: &[
            port("values", Type::List, "Numbers"),
            opt(
                "default",
                Type::Number,
                "null",
                "Result for an empty list (index -1) instead of failing",
            ),
        ],
        outputs: &[
            port("result", Type::Number, "The largest number"),
            port("index", Type::Integer, "Its position (0 is the first)"),
        ],
        errors: &["empty_list"],
        run: max,
    },
    Operation {
        id: "std.collection.sort",
        name: "Sort",
        description: "Numbers or texts in order (ascending unless descending is true)",
        category: "Collections",
        version: crate::VERSION,
        inputs: &[
            LIST,
            opt("descending", Type::Boolean, "false", "Largest first"),
        ],
        outputs: &[port("result", Type::List, "The sorted list")],
        errors: &["invalid_type"],
        run: sort,
    },
    Operation {
        id: "std.collection.contains",
        name: "Contains",
        description: "Whether a list holds an item (numbers by value, lists and objects deeply)",
        category: "Collections",
        version: crate::VERSION,
        inputs: &[LIST, port("item", Type::Any, "The item to look for")],
        outputs: &[port("result", Type::Boolean, "True when found")],
        errors: &[],
        run: contains,
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
        id: "std.object.values",
        name: "Values",
        description: "The values of an object, in key order",
        category: "Objects",
        version: crate::VERSION,
        inputs: &[port("object", Type::Object, "The object")],
        outputs: &[port("result", Type::List, "Values, in key order")],
        errors: &[],
        run: object_values,
    },
    Operation {
        id: "std.object.from_lists",
        name: "From lists",
        description: "An object pairing each key with the value at the same position",
        category: "Objects",
        version: crate::VERSION,
        inputs: &[
            port("keys", Type::List, "Keys (texts, no repeats)"),
            port("values", Type::List, "Values, same length"),
        ],
        outputs: &[port("result", Type::Object, "The object")],
        errors: &["invalid_input"],
        run: object_from_lists,
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
    fn largest() {
        let out = crate::run("std.collection.max", json!({"values": [0, 5, 2, 5]})).unwrap();
        assert_eq!(
            (out["result"].clone(), out["index"].clone()),
            (json!(5), json!(1)),
            "first max wins"
        );
        let out = crate::run("std.collection.max", json!({"values": [1, 2.5, -3]})).unwrap();
        assert_eq!(
            (out["result"].clone(), out["index"].clone()),
            (json!(2.5), json!(1))
        );
        let out = crate::run("std.collection.max", json!({"values": [-4, -2]})).unwrap();
        assert_eq!(out["index"], 1);
        assert_eq!(
            code("std.collection.max", json!({"values": []})),
            "empty_list"
        );
        let out = crate::run("std.collection.max", json!({"values": [], "default": 0})).unwrap();
        assert_eq!(
            (out["result"].clone(), out["index"].clone()),
            (json!(0), json!(-1))
        );
        let out = crate::run("std.collection.max", json!({"values": [3], "default": 0})).unwrap();
        assert_eq!(out["result"], 3, "default only applies to an empty list");
        assert_eq!(
            code("std.collection.max", json!({"values": [1, "x"]})),
            "invalid_type"
        );
    }

    #[test]
    fn sorting() {
        assert_eq!(
            res("std.collection.sort", json!({"list": [3, 1.5, 9, -2]})),
            json!([-2, 1.5, 3, 9])
        );
        assert_eq!(
            res(
                "std.collection.sort",
                json!({"list": [2, 9, 1, 9], "descending": true})
            ),
            json!([9, 9, 2, 1])
        );
        assert_eq!(
            res("std.collection.sort", json!({"list": ["b", "a"]})),
            json!(["a", "b"])
        );
        assert_eq!(res("std.collection.sort", json!({"list": []})), json!([]));
        assert_eq!(
            code("std.collection.sort", json!({"list": [1, "a"]})),
            "invalid_type"
        );
    }

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
            res("std.collection.contains", json!({"list": l, "item": "b"})),
            true
        );
        assert_eq!(
            res("std.collection.contains", json!({"list": l, "item": "B"})),
            false
        );
        assert_eq!(
            res(
                "std.collection.contains",
                json!({"list": [1, 2.0], "item": 2})
            ),
            true
        );
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
            res("std.object.values", json!({"object": {"b": 2, "a": 1}})),
            json!([1, 2]),
            "key order"
        );
        assert_eq!(
            res(
                "std.object.from_lists",
                json!({"keys": ["x", "y"], "values": [1, [2]]})
            ),
            json!({"x": 1, "y": [2]})
        );
        for bad in [
            json!({"keys": ["x"], "values": []}),
            json!({"keys": ["x", "x"], "values": [1, 2]}),
            json!({"keys": [1], "values": [1]}),
        ] {
            assert_eq!(code("std.object.from_lists", bad), "invalid_input");
        }
        assert_eq!(
            res(
                "std.object.set",
                json!({"object": {}, "key": "k", "value": [1]})
            ),
            json!({"k": [1]})
        );
    }
}
