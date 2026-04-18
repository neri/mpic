//! Lempel-Ziv (LZ) compression algorithm supports
use core::num::NonZero;
pub mod lazy;

/// Computes the length of the longest match between `src[lhs..]` and `src[rhs..]`, up to `max_len`.
#[inline]
pub fn matching_len(src: &[u8], lhs: usize, rhs: usize, max_len: usize) -> usize {
    let mut len = 0;
    while len < max_len {
        let lhs = match src.get(lhs + len) {
            Some(v) => *v,
            None => return len,
        };
        let rhs = match src.get(rhs + len) {
            Some(v) => *v,
            None => return len,
        };
        if lhs != rhs {
            return len;
        }
        len += 1;
    }
    len
}

/// Compression level for LZ compression
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum CompressionLevel {
    /// The fastest possible compression
    Fast,
    /// The default compression level
    Default,
    /// The best possible compression
    Best,
}

/// Matching distance and length
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub len: NonZero<usize>,
    pub distance: NonZero<usize>,
}

impl Match {
    #[inline]
    pub const fn new(len: NonZero<usize>, distance: NonZero<usize>) -> Self {
        Self { len, distance }
    }

    /// Clip the length of the match to the given limit
    #[inline]
    pub fn clip_len(&mut self, limit: NonZero<usize>) {
        if self.len > limit {
            self.len = limit;
        }
    }
}

/// Either a match or no match found
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BestMatch {
    /// No match found
    #[default]
    Empty,
    /// A match found
    Found(Match),
}

impl BestMatch {
    /// Create a new `BestMatch` from the given length and distance, which may be zero (no match found).
    #[inline]
    pub const fn new(len: usize, distance: usize) -> Self {
        let len = NonZero::new(len);
        let distance = NonZero::new(distance);
        if let Some(len) = len
            && let Some(distance) = distance
        {
            Self::Found(Match { len, distance })
        } else {
            Self::Empty
        }
    }

    /// Length of the best match, or 0 if no match found
    #[inline]
    pub const fn len(&self) -> usize {
        match self {
            Self::Found(m) => m.len.get(),
            Self::Empty => 0,
        }
    }

    /// Clip the length of the best match to the given limit
    #[inline]
    pub fn clip_len(&mut self, limit: NonZero<usize>) {
        if let Self::Found(m) = self {
            m.clip_len(limit);
        }
    }

    /// Distance of the best match, or 0 if no match found
    #[inline]
    pub const fn distance(&self) -> usize {
        match self {
            Self::Found(m) => m.distance.get(),
            Self::Empty => 0,
        }
    }

    /// Whether the best match is empty (no match found)
    #[inline]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Whether the best match is not empty (a match found)
    #[inline]
    pub const fn get(&self) -> Option<Match> {
        match self {
            Self::Found(m) => Some(*m),
            Self::Empty => None,
        }
    }
}

impl From<Match> for BestMatch {
    #[inline]
    fn from(m: Match) -> Self {
        Self::Found(m)
    }
}

/// LZSS item, either a literal byte or a match
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LZSS {
    Literal(u8),
    Match(Match),
}

impl From<Match> for LZSS {
    #[inline]
    fn from(m: Match) -> Self {
        Self::Match(m)
    }
}

impl From<u8> for LZSS {
    #[inline]
    fn from(v: u8) -> Self {
        Self::Literal(v)
    }
}
