<div align="center">
  <h1>flux-json</h1>
  <p>A JSON parser and serializer.</p>
  <p>
    <img src="https://img.shields.io/badge/release-v.03-e8c37a?style=flat-square" alt="v.03" />
    <img src="https://img.shields.io/badge/status-Proof_of_Concept-d4a359?style=flat-square" alt="Proof of Concept" />
    <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MPL--2.0-blue?style=flat-square" alt="MPL-2.0" /></a>
    <a href="./SECURITY.md"><img src="https://img.shields.io/badge/security-policy-2ea44f?style=flat-square" alt="Security" /></a>
    <img src="https://img.shields.io/badge/rust-2024-orange?style=flat-square" alt="Rust 2024" />
  </p>
</div>

Version 0.3.0.

The browser uses it for profiles, the worker pipe, and script values.

## What it does

- `parse_json` and `parse_json_bounded` read RFC 8259 text: null, booleans, numbers, strings (including `\u` escapes), arrays, and objects.
- `to_json_string` writes compact text. `to_json_pretty` writes indented text.
- Object values have field helpers: `get_str`, `get_bool`, `get_i64`, `get_u64`, `get_array`, `get_object`.

## Limits

- Default nesting depth is 64 (`MAX_JSON_DEPTH`).
- Default input size is 16 MiB (`MAX_JSON_BYTES`).
- `parse_json_bounded` takes a tighter depth and size when the caller wants one.

## Build

```
cargo test
```

Rust 2024. No crates.io dependencies.

## The other repositories

The browser is [flux-browser](https://github.com/buntatoes/flux-browser). These libraries sit next to it. None of them come from crates.io.

| Repository | What it is |
| --- | --- |
| [flux-cipher](https://github.com/buntatoes/flux-cipher) | AES-128, AES-256, CTR, AES-256-GCM |
| [flux-compress](https://github.com/buntatoes/flux-compress) | DEFLATE, zlib, gzip |
| [flux-image](https://github.com/buntatoes/flux-image) | PNG, JPEG, GIF, WebP, ICO, BMP, PPM |
| [flux-json](https://github.com/buntatoes/flux-json) | JSON |
| [flux-ttf](https://github.com/buntatoes/flux-ttf) | TrueType and OpenType |
| [flux-gpu](https://github.com/buntatoes/flux-gpu) | CPU drawing of boxes, shadows, and NV12 |
| [flux-video](https://github.com/buntatoes/flux-video) | CPU frames in a small MP4 container |

## License

MPL-2.0. Copyright (c) 2026 Buntos.
