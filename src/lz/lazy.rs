//! Lazy matching for LZSS compression

use super::*;
use alloc::vec::Vec;
use core::ops::Range;

/// Lazy matching solver for LZSS compression,
/// which allows for better compression ratios by considering future matches before deciding to emit a match or a literal.
pub struct LazyMatchSolver {
    queue: Vec<LazyLzssItem>,
}

impl LazyMatchSolver {
    /// Creates a new `LazyMatchSolver` with an empty queue.
    #[inline]
    pub const fn new() -> Self {
        Self { queue: Vec::new() }
    }

    /// Pushes a new item (either a literal or a match) into the lazy matching queue.
    #[inline]
    pub fn push(&mut self, item: LazyLzssItem) {
        self.queue.push(item);
    }

    /// Solves the lazy matching queue and emits LZSS items using the provided emitter function.
    pub fn solve<F, E>(&self, level: CompressionLevel, mut emitter: F) -> Result<(), E>
    where
        F: FnMut(LZSS) -> Result<(), E>,
    {
        let mut current = 0;

        if matches!(level, CompressionLevel::Fast) {
            while let Some(item) = self.queue.get(current) {
                match item {
                    LazyLzssItem::Literal(v) => {
                        emitter(LZSS::Literal(*v))?;
                        current += 1;
                    }
                    LazyLzssItem::Match(m) => {
                        emitter(LZSS::Match(m.matches()))?;
                        current += m.matches().len.get();
                        continue;
                    }
                }
            }
            return Ok(());
        }

        loop {
            let mut matches = None;
            while let Some(item) = self.queue.get(current) {
                match item {
                    LazyLzssItem::Literal(lit) => {
                        emitter(LZSS::Literal(*lit))?;
                    }
                    LazyLzssItem::Match(m) => {
                        matches = Some(m);
                        break;
                    }
                }
                current += 1;
            }
            let matches = match matches {
                Some(m) => m,
                None => return Ok(()),
            };

            if matches!(level, CompressionLevel::Default) {
                let next_matches = self.queue.get(current + 1).and_then(|item| match item {
                    LazyLzssItem::Literal(_) => None,
                    LazyLzssItem::Match(m) => Some(m),
                });
                if let Some(next_matches) = next_matches {
                    if next_matches.weight() > matches.weight() {
                        emitter(LZSS::Literal(matches.literal()))?;
                        emitter(LZSS::Match(next_matches.matches()))?;
                        current += 1 + next_matches.matches().len.get();
                        continue;
                    }
                }
                emitter(LZSS::Match(matches.matches()))?;
                current += matches.matches().len.get();
                continue;
            }

            //
            if !self.has_competition(matches, current) {
                emitter(LZSS::Match(matches.matches))?;
                current += matches.matches.len.get();
                continue;
            }

            let mut next = current + 1;
            let mut end = current + matches.matches.len.get();
            while next < end {
                if let Some(LazyLzssItem::Match(m)) = self.queue.get(next) {
                    end = end.max(next + m.matches.len.get());
                }
                next += 1;
            }
            let range = current..end;

            let mut candiates = Vec::with_capacity(end - current);
            for i in range.clone() {
                if let Some(LazyLzssItem::Match(m)) = self.queue.get(i) {
                    candiates.push((i, m));
                }
            }

            let solved = Self::_solve(&range, &candiates);

            let mut solved_index = 0;
            let mut cursor = current;
            while cursor < end {
                match self.queue.get(cursor) {
                    Some(LazyLzssItem::Literal(lit)) => {
                        emitter(LZSS::Literal(*lit))?;
                        cursor += 1;
                    }
                    Some(LazyLzssItem::Match(m)) => {
                        if solved_index < solved.len()
                            && cursor == current + solved[solved_index] as usize
                        {
                            emitter(LZSS::Match(m.matches))?;
                            cursor += m.matches.len.get();
                            solved_index += 1;
                        } else {
                            emitter(LZSS::Literal(m.literal()))?;
                            cursor += 1;
                        }
                    }
                    _ => unreachable!(),
                }
            }

            current = end;
        }
    }

    fn _solve(range: &Range<usize>, candidates: &[(usize, &LazyMatch)]) -> Vec<u8> {
        const LIMIT_LEN: usize = 96;

        let len = candidates.len();
        if len == 0 {
            return Vec::new();
        }

        debug_assert!(len <= LIMIT_LEN);

        let candidates = candidates
            .iter()
            .map(|(position, matches)| {
                let position = *position - range.start;
                LazyMatchCandidate {
                    position,
                    end: position + matches.matches().len.get(),
                }
            })
            .collect::<Vec<_>>();

        let mut next_index = [0usize; LIMIT_LEN];
        let mut best = [0usize; LIMIT_LEN + 1];
        let mut take = [false; LIMIT_LEN];

        for index in (0..len).rev() {
            let candidate = &candidates[index];
            let next = candidates[index + 1..]
                .iter()
                .position(|other| other.position >= candidate.end)
                .map(|offset| index + 1 + offset)
                .unwrap_or(len);
            next_index[index] = next;

            let include = (candidate.end - candidate.position) + best[next];
            let exclude = best[index + 1];
            if include > exclude {
                best[index] = include;
                take[index] = true;
            } else {
                best[index] = exclude;
            }
        }

        let mut result = Vec::with_capacity(len);
        let mut index = 0;
        while index < len {
            if take[index] {
                let candidate = &candidates[index];
                result.push(candidate.position as u8);
                index = next_index[index];
            } else {
                index += 1;
            }
        }
        result
    }

    #[inline]
    fn has_competition(&self, matches: &LazyMatch, position: usize) -> bool {
        self.queue[position..][..matches.matches.len.get()]
            .iter()
            .any(|item| match item {
                LazyLzssItem::Literal(_) => false,
                LazyLzssItem::Match(m) => m.weight() > matches.weight(),
            })
    }
}

/// Represents an item in the lazy matching queue, which can be either a literal byte or a match with its associated weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LazyLzssItem {
    /// A literal byte that is not part of a match.
    Literal(u8),
    /// A match that includes the literal byte, the match information, and its weight for comparison with other matches.
    Match(LazyMatch),
}

impl LazyLzssItem {
    #[inline]
    pub fn new(literal: u8, best_match: BestMatch, weight: usize) -> Self {
        match best_match {
            BestMatch::Found(m) => LazyLzssItem::Match(LazyMatch::new(literal, m, weight)),
            BestMatch::Empty => LazyLzssItem::Literal(literal),
        }
    }

    #[inline]
    pub fn literal(&self) -> u8 {
        match self {
            LazyLzssItem::Literal(lit) => *lit,
            LazyLzssItem::Match(m) => m.literal(),
        }
    }
}

/// Represents a match in the lazy matching queue, containing the literal byte, the actual match information, and its weight for comparison with other matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LazyMatch {
    literal: u8,
    matches: Match,
    weight: usize,
}

impl LazyMatch {
    #[inline]
    pub fn new(literal: u8, matches: Match, weight: usize) -> Self {
        Self {
            literal,
            matches,
            weight,
        }
    }

    #[inline]
    pub const fn literal(&self) -> u8 {
        self.literal
    }

    #[inline]
    pub const fn matches(&self) -> Match {
        self.matches
    }

    #[inline]
    pub const fn weight(&self) -> usize {
        self.weight
    }
}

struct LazyMatchCandidate {
    position: usize,
    end: usize,
}
