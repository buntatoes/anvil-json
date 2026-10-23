# Security Policy

## Proof of Concept (PoC) Notice

`flux-json` is a standalone, zero-dependency **Proof of Concept (PoC)** RFC 8259 JSON parser, value tree, and serializer authored from scratch by **Buntos** as part of the [**Flux Browser**](https://github.com/buntatoes/flux-browser) engine ecosystem.

---

## Supported Versions

| Version | Supported |
| :--- | :---: |
| `v.02` (Latest) | Yes |
| `< v.02` | No |

---

## Reporting a Vulnerability

Please **do not** report security vulnerabilities through public GitHub issues or pull requests.

- **GitHub Private Vulnerability Reporting (Recommended)**: [`https://github.com/buntatoes/flux-json/security/advisories/new`](https://github.com/buntatoes/flux-json/security/advisories/new)
- **Central Ecosystem Security Portal**: [`https://github.com/buntatoes/flux-browser/security/advisories/new`](https://github.com/buntatoes/flux-browser/security/advisories/new)
- **Author Attribution**: **Buntos** ([@buntatoes](https://github.com/buntatoes) · `Buntos <324143321+buntatoes@users.noreply.github.com>`)

*(Note: `324143321+buntatoes@users.noreply.github.com` is a GitHub privacy `noreply` commit-attribution address and does not accept direct inbound email; please use GitHub Private Vulnerability Reporting above to contact the maintainer privately.)*

---

## Security Controls in `flux-json`

- **Recursion & Payload Size Guards (`parse_json_bounded`)**:
  - Enforces `MAX_JSON_DEPTH = 64` and `MAX_JSON_BYTES = 16 MiB` by default, preventing stack-overflow recursion bombs and memory exhaustion across IPC channels, session stores, and script `JSON.parse()` calls.
- **Strict UTF-8 & Surrogate Validation**:
  - Validates `\uXXXX` escape sequences and UTF-16 surrogate pairs in 100% safe Rust.
