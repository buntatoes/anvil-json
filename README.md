<div align="center">
  <h1>flux-json</h1>
  <p><strong>Zero-Dependency Bounded RFC 8259 JSON Parser, Value Tree &amp; Serializer (Proof of Concept)</strong></p>
  <p>
    <a href="https://github.com/buntatoes/flux-json/releases/tag/v.02"><img src="https://img.shields.io/badge/release-v.02-e8c37a?style=flat-square" alt="Release v.02" /></a>
    <img src="https://img.shields.io/badge/status-Proof_of_Concept_(PoC)-d4a359?style=flat-square" alt="Proof of Concept" />
    <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MPL--2.0-blue?style=flat-square" alt="License MPL-2.0" /></a>
    <a href="./SECURITY.md"><img src="https://img.shields.io/badge/security-policy-2ea44f?style=flat-square" alt="Security Policy" /></a>
    <a href="https://github.com/buntatoes/flux-json"><img src="https://img.shields.io/badge/rust-2024_edition-orange?style=flat-square" alt="Rust 2024 Edition" /></a>
  </p>
</div>

`flux-json` is a standalone, zero-dependency Rust library for parsing, querying, mutating, and serializing **JSON** (RFC 8259) documents with deterministic nesting-depth (`MAX_JSON_DEPTH = 64`) and payload-size (`MAX_JSON_BYTES = 16 MiB`) security bounds, built from scratch as part of the [**Flux Browser**](https://github.com/buntatoes/flux-browser) Proof of Concept engine ecosystem.

---

## Features (`v.02`)

- **Bounded Depth & Payload Security Guards (`parse_json_bounded`, `MAX_JSON_DEPTH = 64`, `MAX_JSON_BYTES = 16 MiB`)**:
  - Protects IPC channels, session stores, and script parsers against deeply nested array/object recursion bombs and oversized JSON payloads before allocation.
- **Typed Object Lookup Helpers (`get_str`, `get_bool`, `get_i64`, `get_u64`, `get_array`, `get_object`)**:
  - Ergonomic one-step field accessors on `JsonValue::Object` entries alongside `as_u64`.
- **Complete RFC 8259 Parser & Compact/Pretty Serializer (`parse_json`, `to_json_string`, `to_json_pretty`)**:
  - Parses `null`, booleans, integers, floating-point and scientific notation numbers, UTF-8 strings (including `\uXXXX` and UTF-16 surrogate pairs), arrays, and ordered key-value objects.

---

## Quick Start

```rust
use flux_json::{JsonValue, parse_json, parse_json_bounded, to_json_pretty};

let doc = parse_json(r#"{"app": "Flux", "version": 2, "active": true}"#).unwrap();
assert_eq!(doc.get_str("app"), Some("Flux"));
assert_eq!(doc.get_u64("version"), Some(2));
assert_eq!(doc.get_bool("active"), Some(true));

let bounded = parse_json_bounded(r#"{"ipc": "ok"}"#, 16, 4096).unwrap();
assert_eq!(bounded.get_str("ipc"), Some("ok"));

let formatted = to_json_pretty(&doc);
assert!(formatted.contains("\"app\": \"Flux\""));
```

---

## API Overview

| Function / Method | Description |
| :--- | :--- |
| `parse_json(&str) -> Result<JsonValue, JsonError>` | Parses a JSON string using default bounds (`MAX_JSON_DEPTH = 64`, `MAX_JSON_BYTES = 16 MiB`). |
| `parse_json_bounded(&str, max_depth, max_bytes) -> Result<JsonValue, JsonError>` | Parses a JSON string with custom nesting-depth and byte-length ceilings. |
| `to_json_string(&JsonValue) -> String` | Serializes a `JsonValue` into compact single-line JSON. |
| `to_json_pretty(&JsonValue) -> String` | Serializes a `JsonValue` into 2-space indented multi-line JSON. |
| `JsonValue::get(&str) -> Option<&JsonValue>` | Looks up a field by key on a `JsonValue::Object`. |
| `JsonValue::get_str(&str)` / `get_bool(&str)` / `get_i64(&str)` / `get_u64(&str)` | Typed field accessors on a `JsonValue::Object`. |
| `JsonValue::get_index(usize) -> Option<&JsonValue>` | Looks up an element by index on a `JsonValue::Array`. |
| `JsonValue::insert(key, value)` | Inserts or updates a key-value pair on a `JsonValue::Object`. |

---

## Building and Testing

```powershell
cargo test
```

---

## Security Policy

See [`SECURITY.md`](./SECURITY.md) for supported versions, recursion/payload bounds, and vulnerability reporting instructions.

## License

Licensed under the Mozilla Public License Version 2.0 ([`MPL-2.0`](./LICENSE)).

## Author

**Buntos** ([@buntatoes](https://github.com/buntatoes))
