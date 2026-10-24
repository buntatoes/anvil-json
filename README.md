# flux-json

A JSON parser and serializer. Version 0.2.0.

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
