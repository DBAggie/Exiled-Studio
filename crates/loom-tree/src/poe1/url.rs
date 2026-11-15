//! The tree URL the official site and PoB read (PoB's `EncodeURL`, version
//! 6), for an edited spec's `<URL>` child (plan, T-h).
//!
//! Bytes: `0 0 0 6`, the class, `secondary << 2 | ascendancy`, the node
//! count, each node as two bytes big-endian (starts excepted, numbers below
//! 65536), the cluster node count and each cluster node's number less
//! 65536, the mastery count and each mastery as its effect then its node.
//! Base64 with `-` and `_` for `+` and `/`, padding kept, after the site's
//! prefix. Nodes go in ascending order (PoB's is hash order; the site reads
//! either).

use crate::graph::{Kind, Tree};
use crate::poe1::Spec;

/// The URL's prefix.
pub const PREFIX: &str = "https://www.pathofexile.com/passive-skill-tree/";

/// The most nodes, cluster nodes, or masteries the format holds.
const MAX_COUNT: usize = 255;

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn push_u16(out: &mut Vec<u8>, v: u32) {
    out.push(((v >> 8) & 0xff) as u8);
    out.push((v & 0xff) as u8);
}

/// The spec's URL; `None` when the format cannot hold it (a class or
/// ascendancy number over a byte, or more than 255 of a list).
#[must_use]
pub fn encode(tree: &Tree, spec: &Spec, secondary: u32) -> Option<String> {
    let class = u8::try_from(spec.class).ok()?;
    let ascendancy = u8::try_from((secondary << 2) | spec.ascendancy).ok()?;
    let mut nodes = Vec::new();
    let mut clusters = Vec::new();
    let mut masteries = Vec::new();
    for &id in &spec.nodes {
        let start = tree
            .node(id)
            .is_some_and(|n| matches!(n.kind, Kind::ClassStart | Kind::AscendancyStart));
        if id >= 65536 {
            clusters.push(id - 65536);
        } else if !start {
            nodes.push(id);
            if let Some(effect) = spec.masteries.get(&id) {
                masteries.push((*effect, id));
            }
        }
    }
    if nodes.len() > MAX_COUNT || clusters.len() > MAX_COUNT || masteries.len() > MAX_COUNT {
        return None;
    }
    if clusters.iter().any(|c| *c > 0xffff) || masteries.iter().any(|(e, _)| *e > 0xffff) {
        return None;
    }
    let mut bytes = vec![0, 0, 0, 6, class, ascendancy];
    bytes.push(u8::try_from(nodes.len()).ok()?);
    for n in &nodes {
        push_u16(&mut bytes, *n);
    }
    bytes.push(u8::try_from(clusters.len()).ok()?);
    for c in &clusters {
        push_u16(&mut bytes, *c);
    }
    bytes.push(u8::try_from(masteries.len()).ok()?);
    for (e, n) in &masteries {
        push_u16(&mut bytes, *e);
        push_u16(&mut bytes, *n);
    }
    Some(format!("{PREFIX}{}", base64(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_should_pad_and_use_the_url_alphabet() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(&[0xfb, 0xff]), "-_8=");
    }
}
