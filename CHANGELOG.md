# Changelog

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
