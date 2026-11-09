//! Bounds-checked little-endian cursor over untrusted bytes. Every read returns
//! [`Short`] instead of panicking when the input ends early.

/// A read needed more bytes than were left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Short {
    /// What was being read (for error messages).
    pub field: &'static str,
    /// Offset of the read within the buffer.
    pub offset: usize,
    /// Bytes the read needed.
    pub needed: usize,
    /// Bytes that were left.
    pub available: usize,
}

/// Cursor over a byte slice.
#[derive(Debug, Clone)]
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Offset of the next read.
    pub(crate) const fn pos(&self) -> usize {
        self.pos
    }

    /// Everything not read yet.
    pub(crate) fn rest(&self) -> &'a [u8] {
        self.bytes.get(self.pos..).unwrap_or_default()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.rest().is_empty()
    }

    /// Takes the next `n` bytes.
    pub(crate) fn take(&mut self, field: &'static str, n: usize) -> Result<&'a [u8], Short> {
        let rest = self.rest();
        let taken = rest.get(..n).ok_or(Short {
            field,
            offset: self.pos,
            needed: n,
            available: rest.len(),
        })?;
        // `n <= rest.len()`, so this stays within the buffer length.
        self.pos += n;
        Ok(taken)
    }

    fn array<const N: usize>(&mut self, field: &'static str) -> Result<[u8; N], Short> {
        let (head, _) = self.rest().split_first_chunk::<N>().ok_or(Short {
            field,
            offset: self.pos,
            needed: N,
            available: self.rest().len(),
        })?;
        let head = *head;
        self.pos += N;
        Ok(head)
    }

    pub(crate) fn u32(&mut self, field: &'static str) -> Result<u32, Short> {
        self.array(field).map(u32::from_le_bytes)
    }

    pub(crate) fn u64(&mut self, field: &'static str) -> Result<u64, Short> {
        self.array(field).map(u64::from_le_bytes)
    }

    /// Takes bytes up to the next NUL and skips the NUL.
    pub(crate) fn cstr(&mut self, field: &'static str) -> Result<&'a [u8], Short> {
        let rest = self.rest();
        let len = rest.iter().position(|&b| b == 0).ok_or(Short {
            field,
            offset: self.pos,
            needed: rest.len().saturating_add(1),
            available: rest.len(),
        })?;
        let text = self.take(field, len)?;
        self.pos += 1;
        Ok(text)
    }
}

// `widen` relies on this; a target with a narrower `usize` fails to compile.
const _: () = assert!(usize::BITS >= 32);

/// Widens a stored `u32` to `usize`. Every supported target has a `usize` of at
/// least 32 bits (64-bit hosts and wasm32), so this never truncates.
pub(crate) const fn widen(value: u32) -> usize {
    value as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_should_read_little_endian_integers() {
        let mut r = Reader::new(&[1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(r.u32("a"), Ok(1));
        assert_eq!(r.u64("b"), Ok(2));
        assert!(r.is_empty());
    }

    #[test]
    fn reader_should_report_short_read_without_moving() {
        let mut r = Reader::new(&[1, 2, 3]);
        let err = r.u32("count").unwrap_err();
        assert_eq!(
            err,
            Short {
                field: "count",
                offset: 0,
                needed: 4,
                available: 3
            }
        );
        assert_eq!(r.pos(), 0);
    }

    #[test]
    fn cstr_should_stop_at_nul_and_require_one() {
        let mut r = Reader::new(b"ab\0c");
        assert_eq!(r.cstr("s"), Ok(&b"ab"[..]));
        assert_eq!(r.rest(), b"c");
        assert!(r.cstr("s").is_err());
    }
}
