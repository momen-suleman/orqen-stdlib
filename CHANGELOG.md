# Changelog

## 0.8.0

- `std.collection.contains`: whether a list holds an item (numbers by
  value, lists and objects deeply, text case-sensitive).

## 0.7.0

- `std.collection.max` takes an optional `default`: an empty list then gives
  `result` = default and `index` = -1 instead of failing with `empty_list`.

## 0.6.0

- `std.object.values`: the values of an object, in key order.
- `std.object.from_lists`: an object from a list of keys and a list of
  values at the same positions (lengths must match; keys are unique texts).
- `std.logic.choose`: the `then` value when a condition is true, otherwise
  the `else` value.

## 0.5.0

- `std.collection.sort`: a list of only numbers or only texts in order;
  `descending` puts the largest first; mixed lists are `invalid_type`.

## 0.4.0

- `std.collection.max`: the largest number in a list (`result`) and its
  position (`index`); the first one wins a tie; an empty list is `empty_list`.

## 0.3.0

- `std.number.sum`: add up a list of numbers; exact for integers (overflow
  is an error), an empty list is 0, a non-number item is `invalid_type`.

## 0.2.0

- `std.text.contains_any`: whether text contains any of a list of values
  (`result`), and which ones (`matched`); optional `case_sensitive`.
- `std.logic.and` / `std.logic.or` take up to six conditions: `a`, `b` and
  the optional `c`–`f`. Empty extra inputs are neutral (true for And, false
  for Or), so existing workflows behave exactly as before.

## 0.1.0

- 34 typed, deterministic operations: `std.text`, `std.number`,
  `std.logic`, `std.collection`, `std.object`, `std.type`, `std.error`.
- Library workflows `wf.std.text.normalize_input`,
  `wf.std.text.normalize_search_query`, `wf.std.number.calculate_percentage`.
