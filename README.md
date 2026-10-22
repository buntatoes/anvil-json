# flux-json

[![Release](https://img.shields.io/badge/release-v.01-e8c37a)](https://github.com/buntatoes/flux-json/releases/tag/v.01)
[![License: MPL-2.0](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](./LICENSE)
[![Edition](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://github.com/buntatoes/flux-json)

`flux-json` is a standalone, zero-dependency Rust library for parsing, querying, mutating, and serializing **JSON** (RFC 8259) documents.

---

## Features

- **Complete RFC 8259 Parser (`parse_json`)**:
  - Parses `null`, booleans, integers, floating-point and scientific notation numbers, UTF-8 strings, arrays, and ordered key-value objects.
  - Decodes all escape sequences including `\uXXXX` and UTF-16 surrogate pairs.
  - Reports exact byte offsets on malformed input.
- **Ergonomic Value Tree (`JsonValue`)**:
  - Query helpers (`get`, `get_index`, `as_str`, `as_f64`, `as_i64`, `as_bool`, `as_array`, `as_object`) and mutation helpers (`insert`, `push`).
- **Compact & Pretty Serialization (`to_json_string` / `to_json_pretty`)**:
  - Single-line compact output or 2-space indented human-readable formatting.

---

## Quick Start

```rust
use flux_json::{JsonValue, parse_json, to_json_pretty};

let doc = parse_json(r#"{"app": "Flux", "version": 1, "active": true}"#).unwrap();
assert_eq!(doc.get("app").and_then(JsonValue::as_str), Some("Flux"));
assert_eq!(doc.get("version").and_then(JsonValue::as_i64), Some(1));

let formatted = to_json_pretty(&doc);
assert!(formatted.contains("\"app\": \"Flux\""));
```

---

## API Overview

| Function / Method | Description |
| :--- | :--- |
| `parse_json(&str) -> Result<JsonValue, JsonError>` | Parses a JSON string into a `JsonValue` tree. |
| `to_json_string(&JsonValue) -> String` | Serializes a `JsonValue` into compact single-line JSON. |
| `to_json_pretty(&JsonValue) -> String` | Serializes a `JsonValue` into 2-space indented multi-line JSON. |
| `JsonValue::get(&str) -> Option<&JsonValue>` | Looks up a field by key on a `JsonValue::Object`. |
| `JsonValue::get_index(usize) -> Option<&JsonValue>` | Looks up an element by index on a `JsonValue::Array`. |
| `JsonValue::insert(key, value)` | Inserts or updates a key-value pair on a `JsonValue::Object`. |

---

## Building and Testing

```powershell
cargo test
```

---

## License

Licensed under the Mozilla Public License Version 2.0 ([`MPL-2.0`](./LICENSE)).

## Author

**Buntos**
