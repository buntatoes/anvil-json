// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0-OR-Commercial
// Copyright (c) 2026 Buntato
//! Robustness checks for the JSON parser and writer.
//!
//! Inputs come from a seeded generator, so a failure names a case number
//! that reproduces on every run. Each test stops after a short time budget.
//! The parser may reject any of these inputs. It must not panic or hang, and
//! anything it accepts must survive a write and a second parse.

use anvil_json::{JsonValue, parse_json, parse_json_bounded, to_json_pretty, to_json_string};

// ---- shared harness (same text in each repo's robustness test) ----

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Wall-clock budget for one robustness test. Cases run until it is spent.
/// Set `ANVIL_ROBUSTNESS_MS` for a longer local soak run.
fn test_budget() -> Duration {
    let millis = std::env::var("ANVIL_ROBUSTNESS_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1500);
    Duration::from_millis(millis)
}

/// One input that runs longer than this counts as a hang.
const CASE_LIMIT: Duration = Duration::from_secs(10);

/// Upper bound on cases per test, so a fast machine does not loop forever.
const MAX_CASES: usize = 200_000;

/// xorshift64 generator. A fixed seed gives the same inputs on every run.
struct Rng(u64);

#[allow(dead_code)] // Not every test file uses every helper.
impl Rng {
    fn new(seed: u64) -> Self {
        Rng((seed ^ 0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Value in `0..n`. Returns 0 when `n` is 0.
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }

    fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }

    fn bytes(&mut self, max_len: usize) -> Vec<u8> {
        let len = self.below(max_len + 1);
        (0..len).map(|_| self.byte()).collect()
    }
}

/// Apply 1 to 8 random edits: bit flips, byte overwrites, boundary values,
/// inserts, deletes, duplicated runs, and truncation.
fn mutate(rng: &mut Rng, seed: &[u8]) -> Vec<u8> {
    let mut out = seed.to_vec();
    let edits = 1 + rng.below(8);
    for _ in 0..edits {
        let len = out.len();
        match rng.below(8) {
            0 if len > 0 => {
                let i = rng.below(len);
                out[i] ^= 1 << rng.below(8);
            }
            1 if len > 0 => {
                let i = rng.below(len);
                out[i] = rng.byte();
            }
            2 if len > 0 => {
                let i = rng.below(len);
                out[i] = rng.pick(&[0x00, 0x01, 0x7F, 0x80, 0xFE, 0xFF]);
            }
            3 if len >= 4 => {
                let i = rng.below(len - 3);
                let v = rng.pick(&[0u32, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF, 0xFFFF]);
                out[i..i + 4].copy_from_slice(&v.to_be_bytes());
            }
            4 => {
                let i = rng.below(len + 1);
                let extra = rng.bytes(16);
                out.splice(i..i, extra);
            }
            5 if len > 0 => {
                let i = rng.below(len);
                let n = 1 + rng.below((len - i).min(32));
                out.drain(i..i + n);
            }
            6 if len > 0 => {
                let i = rng.below(len);
                let n = 1 + rng.below((len - i).min(64));
                let run = out[i..i + n].to_vec();
                let at = rng.below(len + 1);
                out.splice(at..at, run);
            }
            _ => {
                let keep = rng.below(len + 1);
                out.truncate(keep);
            }
        }
    }
    out
}

fn preview(input: &[u8]) -> String {
    let shown: Vec<String> = input.iter().take(96).map(|b| format!("{b:02x}")).collect();
    let more = if input.len() > 96 { " ..." } else { "" };
    format!("{} bytes: {}{}", input.len(), shown.join(" "), more)
}

/// Run `target` on `input` in its own thread. Fails the test on a panic or
/// when the case runs past [`CASE_LIMIT`]. A hung thread is left behind and
/// ends with the test process.
fn check_case(label: &str, case: usize, input: Vec<u8>, target: fn(&[u8])) {
    let (done_tx, done_rx) = mpsc::channel();
    let shared = std::sync::Arc::new(input);
    let worker_input = shared.clone();
    let handle = thread::Builder::new()
        .name(format!("{label}-{case}"))
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            target(&worker_input);
            let _ = done_tx.send(());
        })
        .expect("spawn case thread");
    match done_rx.recv_timeout(CASE_LIMIT) {
        Ok(()) => {
            let _ = handle.join();
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            let _ = handle.join();
            panic!(
                "{label}: case {case} panicked on input {}",
                preview(&shared)
            );
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!(
                "{label}: case {case} did not finish within {:?} on input {}",
                CASE_LIMIT,
                preview(&shared)
            );
        }
    }
}

/// Generate cases from `seed` until the budget is spent and check each one.
fn run_cases(
    label: &str,
    seed: u64,
    mut generate: impl FnMut(&mut Rng) -> Vec<u8>,
    target: fn(&[u8]),
) {
    let mut rng = Rng::new(seed);
    let budget = test_budget();
    let started = Instant::now();
    let mut case = 0;
    while case < MAX_CASES && started.elapsed() < budget {
        let input = generate(&mut rng);
        check_case(label, case, input, target);
        case += 1;
    }
    eprintln!("{label}: {case} cases in {:?}", started.elapsed());
}

const TOKENS: &[&str] = &[
    "{",
    "}",
    "[",
    "]",
    ":",
    ",",
    "\"",
    "\"k\"",
    "\"\\u00e9\"",
    "\"\\ud83d\\ude00\"",
    "\"\\ud800\"",
    "\"\\\"",
    "\\u",
    "\\",
    "true",
    "false",
    "null",
    "nul",
    "0",
    "-0",
    "1.5e10",
    "1e999",
    "-1e-999",
    "12345678901234567890",
    ".5",
    "1.",
    "01",
    "-",
    "+1",
    "e5",
    " ",
    "\n",
    "\t",
    "\u{0}",
    "\u{e9}",
    "\u{1F600}",
    "/",
    "NaN",
    "Infinity",
];

/// Token soup in JSON's alphabet, built so most inputs get past the first byte.
fn token_soup(rng: &mut Rng) -> Vec<u8> {
    let mut out = String::new();
    let count = rng.below(80);
    for _ in 0..count {
        out.push_str(rng.pick(TOKENS));
    }
    out.into_bytes()
}

/// A random but well-formed document, nested up to `depth`.
fn valid_value(rng: &mut Rng, depth: usize, out: &mut String) {
    let choice = if depth == 0 {
        rng.below(4)
    } else {
        rng.below(6)
    };
    match choice {
        0 => out.push_str(rng.pick(&["true", "false", "null"])),
        1 => out.push_str(&format!("{}", rng.next_u64() as i64 as f64 / 1000.0)),
        2 => out.push_str(rng.pick(&["-0.5e-3", "0", "1E+2", "42"])),
        3 => {
            out.push_str(rng.pick(&["\"\"", "\"a\\nb\"", "\"\\u0041\\/\"", "\"\u{e9}\u{1F600}\""]))
        }
        4 => {
            out.push('[');
            for i in 0..rng.below(5) {
                if i > 0 {
                    out.push(',');
                }
                valid_value(rng, depth - 1, out);
            }
            out.push(']');
        }
        _ => {
            out.push('{');
            for i in 0..rng.below(5) {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&format!("\"k{}\":", rng.below(4)));
                valid_value(rng, depth - 1, out);
            }
            out.push('}');
        }
    }
}

/// Parse, then check that what was accepted writes back to text the parser
/// takes again, and that a second write gives the same text.
fn parse_and_round_trip(input: &[u8]) {
    let text = String::from_utf8_lossy(input);
    let Ok(value) = parse_json(&text) else {
        return;
    };
    for written in [to_json_string(&value), to_json_pretty(&value)] {
        let again = parse_json(&written)
            .unwrap_or_else(|err| panic!("written JSON did not parse again: {err} in {written:?}"));
        assert_eq!(to_json_string(&again), to_json_string(&value));
    }
}

#[test]
fn random_bytes_do_not_panic() {
    run_cases(
        "json-random",
        11,
        |rng| rng.bytes(128),
        parse_and_round_trip,
    );
}

#[test]
fn token_soup_does_not_panic() {
    run_cases("json-tokens", 12, token_soup, parse_and_round_trip);
}

#[test]
fn mutated_documents_do_not_panic() {
    run_cases(
        "json-mutated",
        13,
        |rng| {
            let mut doc = String::new();
            valid_value(rng, 4, &mut doc);
            mutate(rng, doc.as_bytes())
        },
        parse_and_round_trip,
    );
}

#[test]
fn valid_documents_round_trip() {
    run_cases(
        "json-valid",
        14,
        |rng| {
            let mut doc = String::new();
            valid_value(rng, 5, &mut doc);
            doc.into_bytes()
        },
        |input| {
            let text = std::str::from_utf8(input).unwrap();
            let value = parse_json(text)
                .unwrap_or_else(|err| panic!("valid JSON rejected: {err} in {text:?}"));
            assert_eq!(parse_json(&to_json_string(&value)).unwrap(), value);
        },
    );
}

#[test]
fn deep_nesting_is_refused_not_overflowed() {
    for open in ["[", "{\"a\":"] {
        let deep = open.repeat(100_000);
        assert!(parse_json(&deep).is_err());
        assert!(parse_json_bounded(&deep, 8, usize::MAX).is_err());
    }
    // A writer has no depth cap, so a tree at the parse cap must still write.
    let mut value = JsonValue::Null;
    for _ in 0..anvil_json::MAX_JSON_DEPTH {
        let mut outer = JsonValue::empty_array();
        outer.push(value);
        value = outer;
    }
    let written = to_json_string(&value);
    assert_eq!(parse_json(&written).unwrap(), value);
}
