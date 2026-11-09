//! Path hash of the Bundles2 index.
//!
//! The index stores 64-bit hashes of file and directory paths. Current
//! patches use MurmurHash64A (Austin Appleby, public domain) with seed
//! [`PATH_HASH_SEED`] over the lowercase path; older patches used FNV-1a.
//! Confirmed in Spike A (PR A2) by rebuilding every path of the PoE1 and PoE2
//! installs: the hashes match every file record and every directory record
//! (`docs/spikes/spike-a-extraction.md`, section 4).

/// Seed the game uses for path hashes.
pub const PATH_HASH_SEED: u64 = 0x1337_b33f;

/// MurmurHash64A of `data` with `seed`, written from the published algorithm.
#[must_use]
pub fn murmur64a(data: &[u8], seed: u64) -> u64 {
    const M: u64 = 0xc6a4_a793_5bd1_e995;
    const R: u32 = 47;

    // usize is at most 64 bits on every supported target, so this is lossless.
    let len = data.len() as u64;
    let mut h = seed ^ len.wrapping_mul(M);

    let (blocks, tail) = data.as_chunks::<8>();
    for block in blocks {
        let mut k = u64::from_le_bytes(*block).wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        h ^= k;
        h = h.wrapping_mul(M);
    }
    if !tail.is_empty() {
        // Tail bytes little-endian: tail[0] is the lowest byte.
        h ^= tail
            .iter()
            .rev()
            .fold(0_u64, |acc, &b| (acc << 8) | u64::from(b));
        h = h.wrapping_mul(M);
    }

    h ^= h >> R;
    h = h.wrapping_mul(M);
    h ^= h >> R;
    h
}

/// Index hash of a file or directory path: MurmurHash64A over the path with
/// ASCII letters lowercased. Directory paths have no trailing slash; the root
/// directory is the empty string.
#[must_use]
pub fn path_hash(path: &str) -> u64 {
    if path.bytes().any(|b| b.is_ascii_uppercase()) {
        murmur64a(path.to_ascii_lowercase().as_bytes(), PATH_HASH_SEED)
    } else {
        murmur64a(path.as_bytes(), PATH_HASH_SEED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paths and hashes read from the installed PoE1 index (Steam build
    /// 26474023): directory records for the directories, file records for the
    /// files. Together they cover every tail length 0 to 7 and multi-block input.
    const GAME_VECTORS: &[(&str, u64)] = &[
        ("", 0xf42a_94e6_9cff_42fe),
        ("art", 0x9a6c_9521_13c4_42aa),
        ("metadata", 0x5e97_c911_4ce3_78d5),
        ("art/2dart", 0x6d2c_1253_f3e4_6fde),
        ("art/models", 0xda2c_60a7_948d_548e),
        ("art/2ditems", 0x582a_6f22_d0f3_ba82),
        ("art/textures", 0x0c3d_2d69_5559_5c3f),
        ("art/particles", 0x409f_35cc_1074_3533),
        ("art/models/mtx", 0x1dbf_13a8_42b7_a654),
        ("art/2dart/atlas", 0x58fd_3c70_b896_f89d),
        ("data/mods.datc64", 0xfa95_9799_7983_03b7),
        ("data/stats.datc64", 0x5091_5701_cc23_96d8),
    ];

    #[test]
    fn path_hash_should_match_game_index_vectors() {
        for &(path, expected) in GAME_VECTORS {
            assert_eq!(path_hash(path), expected, "path {path:?}");
        }
    }

    #[test]
    fn path_hash_should_ignore_ascii_case() {
        assert_eq!(
            path_hash("Data/Stats.datc64"),
            path_hash("data/stats.datc64")
        );
    }

    #[test]
    fn murmur64a_should_depend_on_seed() {
        assert_ne!(murmur64a(b"art", 0), murmur64a(b"art", PATH_HASH_SEED));
        assert_eq!(murmur64a(b"", 0), 0);
    }
}
