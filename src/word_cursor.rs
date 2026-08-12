// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::tables::word::{self as wd, WordCat};

/// The result of a streaming word-boundary traversal step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WordCursorResult {
    /// The next boundary in the requested direction, as a global byte offset.
    Boundary(usize),
    /// Forward traversal needs a chunk beginning at this exact byte offset.
    NextChunk(usize),
    /// Reverse traversal needs a chunk ending at this exact byte offset.
    PrevChunk(usize),
    /// Arbitrary-offset forward traversal needs context ending at this exact byte offset.
    PreContext(usize),
    /// Arbitrary-offset reverse traversal needs context beginning at this exact byte offset.
    PostContext(usize),
    /// There is no further boundary in the requested direction.
    End,
}

/// A violated [`WordCursor`] input contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WordCursorError {
    /// The cursor offset is greater than the document length.
    InvalidOffset,
    /// The supplied chunk is empty before the requested document edge.
    EmptyChunk,
    /// The chunk's global byte range exceeds the document length or overflows `usize`.
    ChunkOutOfBounds,
    /// The chunk starts or ends after the exact requested offset.
    Gap {
        /// The exact offset requested by the cursor.
        expected: usize,
        /// The offset supplied by the caller.
        actual: usize,
    },
    /// The chunk starts or ends before the exact requested offset.
    Overlap {
        /// The exact offset requested by the cursor.
        expected: usize,
        /// The offset supplied by the caller.
        actual: usize,
    },
    /// Traversal direction changed while a boundary query was incomplete.
    WrongDirection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CursorState {
    Idle,
    ForwardPreContext { origin: usize },
    ForwardReplay { origin: usize },
    ForwardSearch,
    ReversePostContext { origin: usize },
    ReverseReplay { origin: usize },
    ReverseSearch,
}

impl CursorState {
    fn direction(self) -> Option<Direction> {
        match self {
            Self::Idle => None,
            Self::ForwardPreContext { .. } | Self::ForwardReplay { .. } | Self::ForwardSearch => {
                Some(Direction::Forward)
            }
            Self::ReversePostContext { .. } | Self::ReverseReplay { .. } | Self::ReverseSearch => {
                Some(Direction::Reverse)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Significant {
    Cat(WordCat),
    Emoji,
}

/// A fixed-size cursor over Unicode word boundaries in non-contiguous UTF-8 chunks.
///
/// Chunks are borrowed only for the duration of a method call and are never retained. A forward
/// chunk must begin at the cursor's requested offset; a reverse chunk must end there. When a
/// boundary is returned, traversal state resets and the caller starts the next query at that
/// boundary. A boundary query may inspect beyond the boundary it eventually returns, so a caller
/// must be prepared to re-supply source bytes for the next query.
///
/// A cursor created or reset at an arbitrary scalar offset first requests bounded context. Forward
/// traversal requests preceding chunks back to document start and then replays chunks forward;
/// reverse traversal requests following chunks to document end and then replays chunks backward.
/// This establishes exact UAX #29 state without retaining source text or guessing at a chunk edge.
///
/// ```rust
/// use unicode_segmentation::{WordCursor, WordCursorResult};
///
/// let text = "a.b";
/// let mut cursor = WordCursor::new(1, text.len()).unwrap();
/// assert_eq!(
///     cursor.next_boundary(&text[1..2], 1).unwrap(),
///     WordCursorResult::PreContext(1),
/// );
/// assert_eq!(
///     cursor.next_boundary(&text[..1], 0).unwrap(),
///     WordCursorResult::NextChunk(0),
/// );
/// assert_eq!(
///     cursor.next_boundary(text, 0).unwrap(),
///     WordCursorResult::Boundary(3),
/// );
/// ```
#[derive(Clone, Debug)]
pub struct WordCursor {
    offset: usize,
    len: usize,
    state: CursorState,
    known_boundary: bool,
    forward: ForwardState,
    reverse: ReverseState,
}

impl WordCursor {
    /// Creates a cursor at `offset` in a document whose UTF-8 byte length is `len`.
    ///
    /// Both values are global byte offsets. Because no source is supplied here, the caller is
    /// responsible for choosing an offset on a UTF-8 scalar boundary; subsequent exact chunk
    /// contracts prevent the cursor from silently accepting gaps or overlap.
    pub fn new(offset: usize, len: usize) -> Result<Self, WordCursorError> {
        if offset > len {
            return Err(WordCursorError::InvalidOffset);
        }
        Ok(Self {
            offset,
            len,
            state: CursorState::Idle,
            known_boundary: offset == 0 || offset == len,
            forward: ForwardState::new(offset),
            reverse: ReverseState::new(offset),
        })
    }

    /// Returns the cursor's current global byte offset.
    pub fn cur_cursor(&self) -> usize {
        self.offset
    }

    /// Resets the cursor to another byte offset in the same document.
    pub fn set_cursor(&mut self, offset: usize) -> Result<(), WordCursorError> {
        if offset > self.len {
            return Err(WordCursorError::InvalidOffset);
        }
        self.offset = offset;
        self.state = CursorState::Idle;
        self.known_boundary = offset == 0 || offset == self.len;
        self.forward = ForwardState::new(offset);
        self.reverse = ReverseState::new(offset);
        Ok(())
    }

    /// Finds the next word boundary using a chunk that begins at `chunk_start`.
    ///
    /// On [`WordCursorResult::NextChunk`], retry with a non-empty chunk beginning at the returned
    /// offset. [`WordCursorResult::PreContext`] requests a non-empty chunk ending at the returned
    /// offset. A chunk edge before the document end is never interpreted as a word boundary.
    pub fn next_boundary(
        &mut self,
        chunk: &str,
        chunk_start: usize,
    ) -> Result<WordCursorResult, WordCursorError> {
        self.begin(Direction::Forward)?;

        match self.state {
            CursorState::Idle => {
                self.validate_forward_chunk(chunk, chunk_start)?;
                if self.offset == self.len {
                    return Ok(WordCursorResult::End);
                }
                if !self.known_boundary {
                    let origin = self.offset;
                    self.state = CursorState::ForwardPreContext { origin };
                    return Ok(WordCursorResult::PreContext(origin));
                }
                self.state = CursorState::ForwardSearch;
            }
            CursorState::ForwardPreContext { origin } => {
                self.validate_reverse_chunk(chunk, chunk_start)?;
                self.offset = chunk_start;
                if self.offset == 0 {
                    self.forward = ForwardState::new(0);
                    self.state = CursorState::ForwardReplay { origin };
                    return Ok(WordCursorResult::NextChunk(0));
                }
                return Ok(WordCursorResult::PreContext(self.offset));
            }
            CursorState::ForwardReplay { .. } | CursorState::ForwardSearch => {
                self.validate_forward_chunk(chunk, chunk_start)?;
            }
            _ => unreachable!("direction was checked above"),
        }

        let replay_origin = match self.state {
            CursorState::ForwardReplay { origin } => Some(origin),
            _ => None,
        };

        for (relative, ch) in chunk.char_indices() {
            let at = chunk_start + relative;
            if let Some(boundary) = self.forward.push(ch, at) {
                if replay_origin.is_some_and(|origin| boundary <= origin) {
                    self.forward = ForwardState::new(boundary);
                    self.offset = boundary;
                    return Ok(WordCursorResult::NextChunk(boundary));
                }
                return Ok(self.finish_boundary(boundary));
            }
            self.offset = at + ch.len_utf8();
        }

        if self.offset == self.len {
            let boundary = self.forward.finish(self.len);
            if replay_origin.is_some_and(|origin| boundary <= origin) {
                self.forward = ForwardState::new(boundary);
                self.offset = boundary;
                return Ok(WordCursorResult::NextChunk(boundary));
            }
            Ok(self.finish_boundary(boundary))
        } else {
            Ok(WordCursorResult::NextChunk(self.offset))
        }
    }

    /// Finds the previous word boundary using a chunk beginning at `chunk_start` and ending at
    /// the cursor's exact requested offset.
    ///
    /// On [`WordCursorResult::PrevChunk`], retry with a non-empty chunk ending at the returned
    /// offset. [`WordCursorResult::PostContext`] requests a non-empty chunk beginning at the
    /// returned offset. A chunk edge after document start is never interpreted as a word boundary.
    pub fn prev_boundary(
        &mut self,
        chunk: &str,
        chunk_start: usize,
    ) -> Result<WordCursorResult, WordCursorError> {
        self.begin(Direction::Reverse)?;

        match self.state {
            CursorState::Idle => {
                self.validate_reverse_chunk(chunk, chunk_start)?;
                if self.offset == 0 {
                    return Ok(WordCursorResult::End);
                }
                if !self.known_boundary {
                    let origin = self.offset;
                    self.state = CursorState::ReversePostContext { origin };
                    return Ok(WordCursorResult::PostContext(origin));
                }
                self.state = CursorState::ReverseSearch;
            }
            CursorState::ReversePostContext { origin } => {
                self.validate_forward_chunk(chunk, chunk_start)?;
                self.offset = chunk_start + chunk.len();
                if self.offset == self.len {
                    self.reverse = ReverseState::new(self.len);
                    self.state = CursorState::ReverseReplay { origin };
                    return Ok(WordCursorResult::PrevChunk(self.len));
                }
                return Ok(WordCursorResult::PostContext(self.offset));
            }
            CursorState::ReverseReplay { .. } | CursorState::ReverseSearch => {
                self.validate_reverse_chunk(chunk, chunk_start)?;
            }
            _ => unreachable!("direction was checked above"),
        }

        let replay_origin = match self.state {
            CursorState::ReverseReplay { origin } => Some(origin),
            _ => None,
        };

        for (relative, ch) in chunk.char_indices().rev() {
            let at = chunk_start + relative;
            if let Some(boundary) = self.reverse.push(ch, at) {
                if replay_origin.is_some_and(|origin| boundary >= origin) {
                    self.reverse = ReverseState::new(boundary);
                    self.offset = boundary;
                    return Ok(WordCursorResult::PrevChunk(boundary));
                }
                return Ok(self.finish_boundary(boundary));
            }
            self.offset = at;
        }

        if self.offset == 0 {
            let boundary = self.reverse.finish();
            if replay_origin.is_some_and(|origin| boundary >= origin) {
                self.reverse = ReverseState::new(boundary);
                self.offset = boundary;
                return Ok(WordCursorResult::PrevChunk(boundary));
            }
            Ok(self.finish_boundary(boundary))
        } else {
            Ok(WordCursorResult::PrevChunk(self.offset))
        }
    }

    fn begin(&self, direction: Direction) -> Result<(), WordCursorError> {
        match self.state.direction() {
            Some(active) if active != direction => Err(WordCursorError::WrongDirection),
            _ => Ok(()),
        }
    }

    fn finish_boundary(&mut self, boundary: usize) -> WordCursorResult {
        self.offset = boundary;
        self.state = CursorState::Idle;
        self.known_boundary = true;
        self.forward = ForwardState::new(boundary);
        self.reverse = ReverseState::new(boundary);
        WordCursorResult::Boundary(boundary)
    }

    fn validate_forward_chunk(
        &self,
        chunk: &str,
        chunk_start: usize,
    ) -> Result<(), WordCursorError> {
        let chunk_end = chunk_start
            .checked_add(chunk.len())
            .ok_or(WordCursorError::ChunkOutOfBounds)?;
        if chunk_end > self.len {
            return Err(WordCursorError::ChunkOutOfBounds);
        }
        if chunk_start < self.offset {
            return Err(WordCursorError::Overlap {
                expected: self.offset,
                actual: chunk_start,
            });
        }
        if chunk_start > self.offset {
            return Err(WordCursorError::Gap {
                expected: self.offset,
                actual: chunk_start,
            });
        }
        if chunk.is_empty() && self.offset < self.len {
            return Err(WordCursorError::EmptyChunk);
        }
        Ok(())
    }

    fn validate_reverse_chunk(
        &self,
        chunk: &str,
        chunk_start: usize,
    ) -> Result<(), WordCursorError> {
        let chunk_end = chunk_start
            .checked_add(chunk.len())
            .ok_or(WordCursorError::ChunkOutOfBounds)?;
        if chunk_end > self.len {
            return Err(WordCursorError::ChunkOutOfBounds);
        }
        if chunk_end < self.offset {
            return Err(WordCursorError::Gap {
                expected: self.offset,
                actual: chunk_end,
            });
        }
        if chunk_end > self.offset {
            return Err(WordCursorError::Overlap {
                expected: self.offset,
                actual: chunk_end,
            });
        }
        if chunk.is_empty() && self.offset > 0 {
            return Err(WordCursorError::EmptyChunk);
        }
        Ok(())
    }
}

include!("word_cursor_forward.rs");
include!("word_cursor_reverse.rs");
