//! PoB's breakdowns as the oracle writes them (`breakdowns` of an output
//! document: display lines or slot rows, `Modules/CalcBreakdown.lua`,
//! `Modules/CalcDefence.lua`, `Modules/CalcOffence.lua`), read as terms;
//! and our values printed the way PoB prints each line, so a term is
//! compared at the decimals its line shows (Phase 3 plan, decision 9).
//!
//! A line this reader does not know is an error, never a pass: the
//! comparison then fails with `ORACLE_NORMALIZATION` (plan, risks).

use serde_json::Value;

/// How a line prints its number (`docs/phase-3/slice-fields.json`, the
/// first conversion of a term's `line`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// `%g`: 6 significant digits, trailing zeros removed.
    G,
    /// `%.Nf`: N decimals.
    Fixed(usize),
    /// `..` (concatenation): Lua's `tostring`, `%.14g`.
    ToString,
    /// A JSON number written as is: compared exactly.
    Exact,
}

impl Format {
    /// The first conversion in a term's `line` text: `%g`, `%.Nf`, `..`,
    /// or `number` (the whole line). `None` when there is none.
    pub fn of_line(line: &str) -> Option<Self> {
        if line.trim_start().starts_with("number") {
            return Some(Self::Exact);
        }
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i..].starts_with(b"..") {
                return Some(Self::ToString);
            }
            if bytes[i] == b'%' {
                let rest = &line[i + 1..];
                if rest.starts_with('g') {
                    return Some(Self::G);
                }
                if let Some(after) = rest.strip_prefix('.') {
                    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
                    if !digits.is_empty() && after[digits.len()..].starts_with('f') {
                        return digits.parse().ok().map(Self::Fixed);
                    }
                }
            }
            i += 1;
        }
        None
    }

    /// `value` as this format prints it (Lua 5.1 / LuaJIT `string.format`
    /// and `tostring`, which follow C's `printf`).
    pub fn print(self, value: f64) -> String {
        match self {
            Self::G => g(value, 6),
            Self::Fixed(n) => {
                if value.is_finite() {
                    format!("{value:.n$}")
                } else {
                    non_finite(value)
                }
            }
            Self::ToString | Self::Exact => g(value, 14),
        }
    }
}

/// C's `inf`, `-inf`, `nan`, `-nan`.
fn non_finite(value: f64) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    if value.is_nan() {
        format!("{sign}nan")
    } else {
        format!("{sign}inf")
    }
}

/// C's `%.{precision}g`.
pub fn g(value: f64, precision: usize) -> String {
    if !value.is_finite() {
        return non_finite(value);
    }
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.to_owned();
    }
    let p = precision.max(1);
    // The exponent after rounding to p significant digits, as C decides.
    let scientific = format!("{value:.prec$e}", prec = p - 1);
    let Some((mantissa, exponent)) = scientific.split_once('e') else {
        return scientific;
    };
    let x: i64 = exponent.parse().unwrap_or(0);
    let p = i64::try_from(p).unwrap_or(i64::MAX);
    if x < -4 || x >= p {
        let sign = if x < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", strip_zeros(mantissa), x.abs())
    } else {
        let decimals = usize::try_from(p - 1 - x).unwrap_or(0);
        strip_zeros(&format!("{value:.decimals$}"))
    }
}

/// Removes trailing zeros after a decimal point, and the point itself.
fn strip_zeros(text: &str) -> String {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text.to_owned()
    }
}

/// Whether `text` is a number as C prints one: an optional `-`, digits,
/// an optional fraction, an optional exponent; or `inf`/`nan` with an
/// optional `-`.
fn is_number(text: &str) -> bool {
    let t = text.strip_prefix('-').unwrap_or(text);
    if matches!(t, "inf" | "nan") {
        return true;
    }
    let (mantissa, exponent) = match t.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (t, None),
    };
    let (int, frac) = match mantissa.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (mantissa, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    digits(int)
        && frac.is_none_or(digits)
        && exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)))
}

/// One term of a breakdown as PoB printed it.
#[derive(Debug, Clone, PartialEq)]
pub struct Printed {
    /// The term's name (`docs/phase-3/slice-fields.json`).
    pub term: &'static str,
    /// The number as printed; for `(a + b) ^8(base)`, the two parts.
    pub text: Vec<String>,
    /// The line's index (its JSON pointer segment).
    pub index: usize,
}

impl Printed {
    /// The printed number (the sum of the parts, for two).
    pub fn value(&self) -> f64 {
        self.text
            .iter()
            .map(|t| t.parse::<f64>().unwrap_or(f64::NAN))
            .sum()
    }
}

/// The labels of chain lines (`^8(<label>)`) and their terms.
const CHAIN_LABELS: [(&str, &str); 12] = [
    ("base", "base"),
    ("increased/reduced", "increased"),
    ("more/less", "more"),
    ("average damage", "average"),
    ("average hit", "average"),
    ("attack rate", "rate"),
    ("cast rate", "rate"),
    ("trigger rate", "rate"),
    ("hit rate", "rate"),
    ("DPS multiplier for this skill", "dps-multiplier"),
    ("quantity multiplier for this skill", "quantity-multiplier"),
    ("", "total"),
];

/// Reads a `chain` breakdown: `<n> ^8(base)`, `(<n> + <n>) ^8(base)`,
/// `x <n> ^8(<label>)`, `= <n>`. Each term at most once.
pub fn parse_chain(value: &Value) -> Result<Vec<Printed>, String> {
    let lines = value
        .as_array()
        .ok_or_else(|| format!("a chain breakdown is a list of lines, found {value}"))?;
    let mut out: Vec<Printed> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line
            .as_str()
            .ok_or_else(|| format!("line {index} is not text: {line}"))?;
        let printed = chain_line(text, index).ok_or_else(|| format!("line {index}: {text:?}"))?;
        if out.iter().any(|p| p.term == printed.term) {
            return Err(format!("line {index}: a second {} line", printed.term));
        }
        out.push(printed);
    }
    Ok(out)
}

fn chain_line(text: &str, index: usize) -> Option<Printed> {
    let (head, label) = match text.split_once(" ^8(") {
        Some((head, rest)) => (head, Some(rest.strip_suffix(')')?)),
        None => (text, None),
    };
    let (prefix, number) = match head.split_once(' ') {
        Some((p @ ("x" | "="), n)) => (Some(p), n),
        _ => (None, head),
    };
    let term = match (prefix, label) {
        (Some("="), None) => "total",
        (Some("="), Some(_)) => return None,
        (_, Some(label)) => {
            CHAIN_LABELS
                .iter()
                .find(|(l, _)| !l.is_empty() && *l == label)?
                .1
        }
        (_, None) => return None,
    };
    // Factors print with `x`; the base and the average without.
    let factor = !matches!(term, "base" | "average" | "total");
    if factor != (prefix == Some("x")) {
        return None;
    }
    let parts: Vec<String> = match number.strip_prefix('(').and_then(|n| n.strip_suffix(')')) {
        Some(inner) if term == "base" => {
            let (a, b) = inner.split_once(" + ")?;
            vec![a.to_owned(), b.to_owned()]
        }
        Some(_) => return None,
        None => vec![number.to_owned()],
    };
    parts.iter().all(|p| is_number(p)).then_some(Printed {
        term,
        text: parts,
        index,
    })
}

/// Reads a `min-max-total` breakdown: `Min: <n>%`, `Max: <n>%`,
/// `Total: <n>%`, each once.
pub fn parse_min_max_total(value: &Value) -> Result<Vec<Printed>, String> {
    let lines = value
        .as_array()
        .ok_or_else(|| format!("a min-max-total breakdown is a list of lines, found {value}"))?;
    let mut out: Vec<Printed> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line
            .as_str()
            .ok_or_else(|| format!("line {index} is not text: {line}"))?;
        let found = [("Min: ", "min"), ("Max: ", "max"), ("Total: ", "total")]
            .into_iter()
            .find_map(|(prefix, term)| {
                let n = text.strip_prefix(prefix)?.strip_suffix('%')?;
                is_number(n).then(|| Printed {
                    term,
                    text: vec![n.to_owned()],
                    index,
                })
            })
            .ok_or_else(|| format!("line {index}: {text:?}"))?;
        if out.iter().any(|p| p.term == found.term) {
            return Err(format!("line {index}: a second {} line", found.term));
        }
        out.push(found);
    }
    Ok(out)
}

/// One row of a `slots` breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotRow {
    /// The row's index in `slots`.
    pub index: usize,
    /// The row's node name: `step.slot.<source>[.<sourceName>]`.
    pub node: String,
    /// `base`: a JSON number.
    pub base: f64,
    /// `inc`: ` x <n>`, or `None` for `false`.
    pub inc: Option<String>,
    /// `more`: ` x <n>`, or `None` for `false`.
    pub more: Option<String>,
    /// `total`: `<n>`.
    pub total: String,
}

/// The keys a slot row may have (`item` is the item object the adapter
/// writes as an opaque marker).
const ROW_KEYS: [&str; 7] = [
    "base",
    "inc",
    "item",
    "more",
    "source",
    "sourceName",
    "total",
];

/// The graph name of a slot source: lowercased, spaces as `-`.
pub fn slot_name(text: &str) -> String {
    text.trim().to_lowercase().replace(' ', "-")
}

/// Reads a `slots` breakdown: `{"slots": [rows]}`, or `{"slots": {}}`
/// for none (an empty Lua table).
pub fn parse_slots(value: &Value) -> Result<Vec<SlotRow>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("a slots breakdown is an object, found {value}"))?;
    if let Some(other) = object.keys().find(|k| *k != "slots") {
        return Err(format!("unknown key {other:?}"));
    }
    let rows = match object.get("slots") {
        Some(Value::Array(rows)) => rows.as_slice(),
        Some(Value::Object(empty)) if empty.is_empty() => &[],
        other => return Err(format!("slots is not a list: {other:?}")),
    };
    let mut out: Vec<SlotRow> = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let bad = |what: &str| format!("row {index}: {what}");
        let r = row.as_object().ok_or_else(|| bad("not an object"))?;
        if let Some(k) = r.keys().find(|k| !ROW_KEYS.contains(&k.as_str())) {
            return Err(bad(&format!("unknown key {k:?}")));
        }
        let source = r
            .get("source")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| bad("no source"))?;
        let mut node = format!("step.slot.{}", slot_name(source));
        match r.get("sourceName") {
            None => {}
            Some(Value::String(name)) if !name.trim().is_empty() => {
                node.push('.');
                node.push_str(&slot_name(name));
            }
            Some(other) => return Err(bad(&format!("sourceName {other}"))),
        }
        let base = r
            .get("base")
            .and_then(Value::as_f64)
            .ok_or_else(|| bad("base is not a number"))?;
        let factor = |key: &str| -> Result<Option<String>, String> {
            match r.get(key) {
                Some(Value::Bool(false)) => Ok(None),
                Some(Value::String(s)) => s
                    .strip_prefix(" x ")
                    .filter(|n| is_number(n))
                    .map(|n| Some(n.to_owned()))
                    .ok_or_else(|| bad(&format!("{key} {s:?}"))),
                other => Err(bad(&format!("{key} {other:?}"))),
            }
        };
        let total = r
            .get("total")
            .and_then(Value::as_str)
            .filter(|t| is_number(t))
            .ok_or_else(|| bad("total is not a printed number"))?;
        let slot = SlotRow {
            index,
            node,
            base,
            inc: factor("inc")?,
            more: factor("more")?,
            total: total.to_owned(),
        };
        if out.iter().any(|o| o.node == slot.node) {
            return Err(bad(&format!("a second row for {}", slot.node)));
        }
        out.push(slot);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn g_should_print_as_c_does() {
        for (v, text) in [
            (0.0, "0"),
            (-0.0, "-0"),
            (2583.0, "2583"),
            (2583.4, "2583.4"),
            (1_057_055.25, "1.05706e+06"),
            (123_456.0, "123456"),
            (123_456.5, "123456"),
            (123_457.5, "123458"),
            (0.0001, "0.0001"),
            (0.000_012_5, "1.25e-05"),
            (1.0 / 3.0, "0.333333"),
            (-200.0, "-200"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
        ] {
            assert_eq!(g(v, 6), text, "{v}");
        }
        assert_eq!(g(48.290_000_000_000_006, 14), "48.29");
        assert_eq!(g(0.1 + 0.2, 14), "0.3");
        assert_eq!(g(0.1 + 0.2, 17), "0.30000000000000004");
        assert_eq!(g(1e15, 14), "1e+15");
    }

    #[test]
    fn formats_should_come_from_the_line() {
        for (line, format) in [
            ("%g ^8(base)", Some(Format::G)),
            ("x %.2f ^8(increased/reduced)", Some(Format::Fixed(2))),
            ("= %.1f", Some(Format::Fixed(1))),
            ("Min: ..%", Some(Format::ToString)),
            ("number", Some(Format::Exact)),
            (" x %.2f, or false", Some(Format::Fixed(2))),
            ("text", None),
            ("100%", None),
        ] {
            assert_eq!(Format::of_line(line), format, "{line}");
        }
        assert_eq!(Format::Fixed(2).print(1.345), "1.34"); // 1.345 is below 1.345 in binary
        assert_eq!(Format::Fixed(2).print(2.0), "2.00");
        assert_eq!(Format::Fixed(1).print(84_002.149), "84002.1");
        assert_eq!(Format::ToString.print(75.0), "75");
    }

    #[test]
    fn chain_lines_should_read_every_shape() {
        let lines = json!([
            "(1890 + 23) ^8(base)",
            "x 1.35 ^8(increased/reduced)",
            "x 0.90 ^8(more/less)",
            "= 2583"
        ]);
        let p = parse_chain(&lines).unwrap();
        let terms: Vec<&str> = p.iter().map(|t| t.term).collect();
        assert_eq!(terms, ["base", "increased", "more", "total"]);
        assert_eq!(p[0].text, ["1890", "23"]);
        assert_eq!(p[0].value(), 1913.0);
        let dps = json!([
            "21163.3 ^8(average hit)",
            "x 3.97 ^8(cast rate)",
            "x 2 ^8(DPS multiplier for this skill)",
            "x 3 ^8(quantity multiplier for this skill)",
            "= 84002.1"
        ]);
        let terms: Vec<&str> = parse_chain(&dps).unwrap().iter().map(|t| t.term).collect();
        assert_eq!(
            terms,
            [
                "average",
                "rate",
                "dps-multiplier",
                "quantity-multiplier",
                "total"
            ]
        );
    }

    #[test]
    fn chain_should_refuse_unknown_lines() {
        for line in [
            "1913 ^8(flat)",
            "x 1913 ^8(base)",
            "1.35 ^8(increased/reduced)",
            "= 2583 ^8(total)",
            "2583",
            "x abc ^8(more/less)",
            "(1 + 2) ^8(more/less)",
            "^7x 1.35 ^8(increased/reduced)",
        ] {
            assert!(parse_chain(&json!([line])).is_err(), "{line}");
        }
        assert!(parse_chain(&json!(["= 1", "= 2"])).is_err());
        assert!(parse_chain(&json!({"slots": []})).is_err());
        assert!(parse_chain(&json!([1])).is_err());
    }

    #[test]
    fn min_max_total_should_read_its_three_lines() {
        let p = parse_min_max_total(&json!(["Min: -200%", "Max: 75%", "Total: 120%"])).unwrap();
        assert_eq!(p[0].text, ["-200"]);
        assert_eq!(p[2].term, "total");
        for bad in [
            json!(["Min: -200"]),
            json!(["Cap: 75%"]),
            json!(["Max: 75%", "Max: 76%"]),
        ] {
            assert!(parse_min_max_total(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn slots_should_read_rows_and_the_empty_table() {
        let v = json!({"slots": [
            {"base": 58, "inc": " x 1.48", "item": {"$object": "Item"}, "more": false, "source": "Body Armour", "total": "85.84"},
            {"base": 15, "inc": false, "more": " x 1.10", "source": "Conversion", "sourceName": "Life", "total": "16.50"}
        ]});
        let rows = parse_slots(&v).unwrap();
        assert_eq!(rows[0].node, "step.slot.body-armour");
        assert_eq!(rows[0].inc.as_deref(), Some("1.48"));
        assert_eq!(rows[0].more, None);
        assert_eq!(rows[1].node, "step.slot.conversion.life");
        assert_eq!(rows[1].more.as_deref(), Some("1.10"));
        assert!(parse_slots(&json!({"slots": {}})).unwrap().is_empty());
        for bad in [
            json!({"slots": [{"base": 1, "inc": false, "more": false, "total": "1"}]}),
            json!({"slots": [{"base": "1", "inc": false, "more": false, "source": "Global", "total": "1"}]}),
            json!({"slots": [{"base": 1, "inc": "1.2", "more": false, "source": "Global", "total": "1"}]}),
            json!({"slots": [{"base": 1, "inc": false, "more": false, "source": "Global", "total": 1}]}),
            json!({"slots": [{"base": 1, "inc": false, "more": false, "source": "Global", "total": "1", "extra": 1}]}),
            json!({"slots": [], "other": 1}),
            json!({"slots": {"a": 1}}),
            json!(["1"]),
        ] {
            assert!(parse_slots(&bad).is_err(), "{bad}");
        }
    }
}
