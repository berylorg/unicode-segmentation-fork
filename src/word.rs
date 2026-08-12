// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::cmp;
use core::iter::Filter;

/// An iterator over the substrings of a string which, after splitting the string on
/// [word boundaries](http://www.unicode.org/reports/tr29/#Word_Boundaries),
/// contain any characters with the
/// [Alphabetic](http://unicode.org/reports/tr44/#Alphabetic)
/// property, or with
/// [General_Category=Number](http://unicode.org/reports/tr44/#General_Category_Values).
///
/// This struct is created by the [`unicode_words`] method on the [`UnicodeSegmentation`] trait. See
/// its documentation for more.
///
/// [`unicode_words`]: trait.UnicodeSegmentation.html#tymethod.unicode_words
/// [`UnicodeSegmentation`]: trait.UnicodeSegmentation.html
#[derive(Debug)]
pub struct UnicodeWords<'a> {
    inner: WordsIter<'a>,
}

impl<'a> Iterator for UnicodeWords<'a> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            WordsIter::Ascii(i) => i.next(),
            WordsIter::Unicode(i) => i.next(),
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            WordsIter::Ascii(i) => i.size_hint(),
            WordsIter::Unicode(i) => i.size_hint(),
        }
    }
}
impl<'a> DoubleEndedIterator for UnicodeWords<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            WordsIter::Ascii(i) => i.next_back(),
            WordsIter::Unicode(i) => i.next_back(),
        }
    }
}

/// An iterator over the substrings of a string which, after splitting the string on
/// [word boundaries](http://www.unicode.org/reports/tr29/#Word_Boundaries),
/// contain any characters with the
/// [Alphabetic](http://unicode.org/reports/tr44/#Alphabetic)
/// property, or with
/// [General_Category=Number](http://unicode.org/reports/tr44/#General_Category_Values).
/// This iterator also provides the byte offsets for each substring.
///
/// This struct is created by the [`unicode_word_indices`] method on the [`UnicodeSegmentation`] trait. See
/// its documentation for more.
///
/// [`unicode_word_indices`]: trait.UnicodeSegmentation.html#tymethod.unicode_word_indices
/// [`UnicodeSegmentation`]: trait.UnicodeSegmentation.html
#[derive(Debug)]
pub struct UnicodeWordIndices<'a> {
    inner: IndicesIter<'a>,
}

impl<'a> Iterator for UnicodeWordIndices<'a> {
    type Item = (usize, &'a str);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            IndicesIter::Ascii(i) => i.next(),
            IndicesIter::Unicode(i) => i.next(),
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            IndicesIter::Ascii(i) => i.size_hint(),
            IndicesIter::Unicode(i) => i.size_hint(),
        }
    }
}
impl<'a> DoubleEndedIterator for UnicodeWordIndices<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            IndicesIter::Ascii(i) => i.next_back(),
            IndicesIter::Unicode(i) => i.next_back(),
        }
    }
}

/// External iterator for a string's
/// [word boundaries](http://www.unicode.org/reports/tr29/#Word_Boundaries).
///
/// This struct is created by the [`split_word_bounds`] method on the [`UnicodeSegmentation`]
/// trait. See its documentation for more.
///
/// [`split_word_bounds`]: trait.UnicodeSegmentation.html#tymethod.split_word_bounds
/// [`UnicodeSegmentation`]: trait.UnicodeSegmentation.html
#[derive(Debug, Clone)]
pub struct UWordBounds<'a> {
    string: &'a str,
}

/// External iterator for word boundaries and byte offsets.
///
/// This struct is created by the [`split_word_bound_indices`] method on the
/// [`UnicodeSegmentation`] trait. See its documentation for more.
///
/// [`split_word_bound_indices`]: trait.UnicodeSegmentation.html#tymethod.split_word_bound_indices
/// [`UnicodeSegmentation`]: trait.UnicodeSegmentation.html
#[derive(Debug, Clone)]
pub struct UWordBoundIndices<'a> {
    start_offset: usize,
    iter: UWordBounds<'a>,
}

impl<'a> UWordBoundIndices<'a> {
    #[inline]
    /// View the underlying data (the part yet to be iterated) as a slice of the original string.
    ///
    /// ```rust
    /// # use unicode_segmentation::UnicodeSegmentation;
    /// let mut iter = "Hello world".split_word_bound_indices();
    /// assert_eq!(iter.as_str(), "Hello world");
    /// iter.next();
    /// assert_eq!(iter.as_str(), " world");
    /// iter.next();
    /// assert_eq!(iter.as_str(), "world");
    /// ```
    pub fn as_str(&self) -> &'a str {
        self.iter.as_str()
    }
}

impl<'a> Iterator for UWordBoundIndices<'a> {
    type Item = (usize, &'a str);

    #[inline]
    fn next(&mut self) -> Option<(usize, &'a str)> {
        self.iter
            .next()
            .map(|s| (s.as_ptr() as usize - self.start_offset, s))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for UWordBoundIndices<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<(usize, &'a str)> {
        self.iter
            .next_back()
            .map(|s| (s.as_ptr() as usize - self.start_offset, s))
    }
}

impl<'a> Iterator for UWordBounds<'a> {
    type Item = &'a str;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let slen = self.string.len();
        (cmp::min(slen, 1), Some(slen))
    }

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        let mut cursor = crate::word_cursor::WordCursor::new(0, self.string.len()).unwrap();
        let idx = match cursor.next_boundary(self.string, 0).unwrap() {
            crate::word_cursor::WordCursorResult::Boundary(idx) => idx,
            crate::word_cursor::WordCursorResult::End => return None,
            _ => unreachable!("a complete string cannot request another chunk"),
        };
        let (result, remaining) = self.string.split_at(idx);
        self.string = remaining;
        Some(result)
    }
}

impl<'a> DoubleEndedIterator for UWordBounds<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        let mut cursor =
            crate::word_cursor::WordCursor::new(self.string.len(), self.string.len()).unwrap();
        let idx = match cursor.prev_boundary(self.string, 0).unwrap() {
            crate::word_cursor::WordCursorResult::Boundary(idx) => idx,
            crate::word_cursor::WordCursorResult::End => return None,
            _ => unreachable!("a complete string cannot request another chunk"),
        };
        let (remaining, result) = self.string.split_at(idx);
        self.string = remaining;
        Some(result)
    }
}

impl<'a> UWordBounds<'a> {
    #[inline]
    /// View the underlying data (the part yet to be iterated) as a slice of the original string.
    ///
    /// ```rust
    /// # use unicode_segmentation::UnicodeSegmentation;
    /// let mut iter = "Hello world".split_word_bounds();
    /// assert_eq!(iter.as_str(), "Hello world");
    /// iter.next();
    /// assert_eq!(iter.as_str(), " world");
    /// iter.next();
    /// assert_eq!(iter.as_str(), "world");
    /// ```
    pub fn as_str(&self) -> &'a str {
        self.string
    }
}

/// ASCII‑fast‑path word‑boundary iterator for strings that contain only ASCII characters.
///
/// Since we handle only ASCII characters, we can use a much simpler set of
/// word break values than the full Unicode algorithm.
/// https://www.unicode.org/reports/tr29/#Table_Word_Break_Property_Values
///
/// | Word_Break value | ASCII code points that belong to it                             |
/// | -----------------| --------------------------------------------------------------- |
/// | CR               | U+000D (CR)                                                     |
/// | LF               | U+000A (LF)                                                     |
/// | Newline          | U+000B (VT), U+000C (FF)                                        |
/// | Single_Quote     | U+0027 (')                                                      |
/// | Double_Quote     | U+0022 (")                                                      |
/// | MidNumLet        | U+002E (.) FULL STOP                                            |
/// | MidLetter        | U+003A (:) COLON                                                |
/// | MidNum           | U+002C (,), U+003B (;)                                          |
/// | Numeric          | U+0030 – U+0039 (0 … 9)                                         |
/// | ALetter          | U+0041 – U+005A (A … Z), U+0061 – U+007A (a … z)                |
/// | ExtendNumLet     | U+005F (_) underscore                                           |
/// | WSegSpace        | U+0020 (SPACE)                                                  |
///
/// The macro MidNumLetQ boils down to: U+002E (.) FULL STOP and U+0027 (')
/// AHLetter is the same as ALetter, so we don't need to distinguish it.
///
/// Any other single ASCII byte is its own boundary (the default WB999).
#[derive(Debug)]
struct AsciiWordBoundIter<'a> {
    rest: &'a str,
    offset: usize,
}

impl<'a> AsciiWordBoundIter<'a> {
    pub fn new(s: &'a str) -> Self {
        AsciiWordBoundIter { rest: s, offset: 0 }
    }

    #[inline]
    fn is_core(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }

    #[inline]
    fn is_infix(b: u8, prev: u8, next: u8) -> bool {
        match b {
            // Numeric separators such as "1,000" or "3.14" (WB11/WB12)
            //
            // "Numeric (MidNum | MidNumLetQ) Numeric"
            b'.' | b',' | b';' | b'\'' if prev.is_ascii_digit() && next.is_ascii_digit() => true,

            // Dot or colon inside an alphabetic word ("e.g.", "http://") (WB6/WB7)
            //
            // "(MidLetter | MidNumLetQ) AHLetter (MidLetter | MidNumLetQ)"
            // MidLetter  = b':'
            // MidNumLetQ = b'.' | b'\''
            b'\'' | b'.' | b':' if prev.is_ascii_alphabetic() && next.is_ascii_alphabetic() => true,
            _ => false,
        }
    }
}

impl<'a> Iterator for AsciiWordBoundIter<'a> {
    type Item = (usize, &'a str);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        let bytes = self.rest.as_bytes();
        let len = bytes.len();

        // 1) Keep horizontal whitespace together.
        // Spec: WB3d joins adjacent *WSegSpace* into a single segment.
        if bytes[0] == b' ' {
            let mut i = 1;
            while i < len && bytes[i] == b' ' {
                i += 1;
            }
            let word = &self.rest[..i];
            let pos = self.offset;
            self.rest = &self.rest[i..];
            self.offset += i;
            return Some((pos, word));
        }

        // 2) Core-run (letters/digits/underscore + infix)
        // Spec: ALetter × ALetter, Numeric × Numeric etc. (WB5–WB13b)
        if Self::is_core(bytes[0]) {
            let mut i = 1;
            while i < len {
                let b = bytes[i];
                if Self::is_core(b)
                    || (i + 1 < len && Self::is_infix(b, bytes[i - 1], bytes[i + 1]))
                {
                    i += 1;
                } else {
                    break;
                }
            }
            let word = &self.rest[..i];
            let pos = self.offset;
            self.rest = &self.rest[i..];
            self.offset += i;
            return Some((pos, word));
        }

        // 3) Do not break within CRLF.
        // Spec: WB3 treats CR+LF as a single non‑breaking pair.
        if bytes[0] == b'\r' && len >= 2 && bytes[1] == b'\n' {
            let word = &self.rest[..2];
            let pos = self.offset;
            self.rest = &self.rest[2..];
            self.offset += 2;
            Some((pos, word))
        } else {
            // 4) Otherwise, break everywhere
            // Spec: the catch‑all rule WB999.
            let word = &self.rest[..1];
            let pos = self.offset;
            self.rest = &self.rest[1..];
            self.offset += 1;
            Some((pos, word))
        }
    }
}

impl<'a> DoubleEndedIterator for AsciiWordBoundIter<'a> {
    fn next_back(&mut self) -> Option<(usize, &'a str)> {
        let rest = self.rest;
        if rest.is_empty() {
            return None;
        }
        let bytes = rest.as_bytes();
        let len = bytes.len();

        // 1) Group runs of spaces
        // Spec: WB3d joins adjacent *WSegSpace* into a single segment.
        if bytes[len - 1] == b' ' {
            // find start of this last run of spaces
            let mut start = len - 1;
            while start > 0 && bytes[start - 1] == b' ' {
                start -= 1;
            }
            let word = &rest[start..];
            let pos = self.offset + start;
            self.rest = &rest[..start];
            return Some((pos, word));
        }

        // 2) Trailing Core-run (letters/digits/underscore + infix)
        // Spec: ALetter × ALetter, Numeric × Numeric etc. (WB5–WB13b)
        if Self::is_core(bytes[len - 1]) {
            // scan backwards as long as we see `is_core` or an `is_infix`
            let mut start = len - 1;
            while start > 0 {
                let b = bytes[start - 1];
                let prev = if start >= 2 { bytes[start - 2] } else { b };
                let next = bytes[start]; // the byte we just included
                if Self::is_core(b) || Self::is_infix(b, prev, next) {
                    start -= 1;
                } else {
                    break;
                }
            }
            let word = &rest[start..];
            let pos = self.offset + start;
            self.rest = &rest[..start];
            return Some((pos, word));
        }

        // 3) Non-core: CR+LF as one token, otherwise single char
        // Spec: WB3 treats CR+LF as a single non‑breaking pair.
        if len >= 2 && bytes[len - 2] == b'\r' && bytes[len - 1] == b'\n' {
            let start = len - 2;
            let word = &rest[start..];
            let pos = self.offset + start;
            self.rest = &rest[..start];
            return Some((pos, word));
        }

        // 4) Fallback – every other byte is its own segment
        // Spec: the catch‑all rule WB999.
        let start = len - 1;
        let word = &rest[start..];
        let pos = self.offset + start;
        self.rest = &rest[..start];
        Some((pos, word))
    }
}

#[inline]
fn ascii_word_ok(t: &(usize, &str)) -> bool {
    has_ascii_alphanumeric(&t.1)
}
#[inline]
fn unicode_word_ok(t: &(usize, &str)) -> bool {
    has_alphanumeric(&t.1)
}

type AsciiWordsIter<'a> = Filter<
    core::iter::Map<AsciiWordBoundIter<'a>, fn((usize, &'a str)) -> &'a str>,
    fn(&&'a str) -> bool,
>;
type UnicodeWordsIter<'a> = Filter<UWordBounds<'a>, fn(&&'a str) -> bool>;
type AsciiIndicesIter<'a> = Filter<AsciiWordBoundIter<'a>, fn(&(usize, &'a str)) -> bool>;
type UnicodeIndicesIter<'a> = Filter<UWordBoundIndices<'a>, fn(&(usize, &'a str)) -> bool>;

#[derive(Debug)]
enum WordsIter<'a> {
    Ascii(AsciiWordsIter<'a>),
    Unicode(UnicodeWordsIter<'a>),
}

#[derive(Debug)]
enum IndicesIter<'a> {
    Ascii(AsciiIndicesIter<'a>),
    Unicode(UnicodeIndicesIter<'a>),
}

#[inline]
pub fn new_unicode_words(s: &str) -> UnicodeWords<'_> {
    let inner = if s.is_ascii() {
        WordsIter::Ascii(new_unicode_words_ascii(s))
    } else {
        WordsIter::Unicode(new_unicode_words_general(s))
    };
    UnicodeWords { inner }
}

#[inline]
pub fn new_unicode_word_indices(s: &str) -> UnicodeWordIndices<'_> {
    let inner = if s.is_ascii() {
        IndicesIter::Ascii(new_ascii_word_bound_indices(s).filter(ascii_word_ok))
    } else {
        IndicesIter::Unicode(new_word_bound_indices(s).filter(unicode_word_ok))
    };
    UnicodeWordIndices { inner }
}

#[inline]
pub fn new_word_bounds(s: &str) -> UWordBounds<'_> {
    UWordBounds { string: s }
}

#[inline]
pub fn new_word_bound_indices(s: &str) -> UWordBoundIndices<'_> {
    UWordBoundIndices {
        start_offset: s.as_ptr() as usize,
        iter: new_word_bounds(s),
    }
}

#[inline]
fn new_ascii_word_bound_indices(s: &str) -> AsciiWordBoundIter<'_> {
    AsciiWordBoundIter::new(s)
}

#[inline]
fn has_alphanumeric(s: &&str) -> bool {
    use crate::tables::util::is_alphanumeric;

    s.chars().any(is_alphanumeric)
}

#[inline]
fn has_ascii_alphanumeric(s: &&str) -> bool {
    s.chars().any(|c| c.is_ascii_alphanumeric())
}

#[inline(always)]
fn strip_pos((_, w): (usize, &str)) -> &str {
    w
}

#[inline]
fn new_unicode_words_ascii<'a>(s: &'a str) -> AsciiWordsIter<'a> {
    new_ascii_word_bound_indices(s)
        .map(strip_pos as fn(_) -> _)
        .filter(has_ascii_alphanumeric)
}

#[inline]
fn new_unicode_words_general<'a>(s: &'a str) -> UnicodeWordsIter<'a> {
    new_word_bounds(s).filter(has_alphanumeric)
}

#[cfg(test)]
mod tests {
    use crate::word::{
        new_ascii_word_bound_indices, new_unicode_words_ascii, new_word_bound_indices,
    };
    use std::string::String;
    use std::vec;
    use std::vec::Vec;

    use proptest::prelude::*;

    #[test]
    fn test_syriac_abbr_mark() {
        use crate::tables::word as wd;
        let (_, _, cat) = wd::word_category('\u{70f}');
        assert_eq!(cat, wd::WC_ALetter);
    }

    #[test]
    fn test_end_of_ayah_cat() {
        use crate::tables::word as wd;
        let (_, _, cat) = wd::word_category('\u{6dd}');
        assert_eq!(cat, wd::WC_Numeric);
    }

    #[test]
    fn test_ascii_word_bound_indices_various_cases() {
        let s = "Hello, world!";
        let words: Vec<(usize, &str)> = new_ascii_word_bound_indices(s).collect();
        let expected = vec![
            (0, "Hello"), // simple letters
            (5, ","),
            (6, " "),     // space after comma
            (7, "world"), // skip comma+space, stop at '!'
            (12, "!"),    // punctuation at the end
        ];
        assert_eq!(words, expected);
    }

    #[test]
    fn test_ascii_word_indices_various_cases() {
        let s = "Hello, world! can't e.g. var1 123,456 foo_bar example.com 127.0.0.1:9090";
        let words: Vec<&str> = new_unicode_words_ascii(s).collect();
        let expected = vec![
            ("Hello"), // simple letters
            ("world"), // skip comma+space, stop at '!'
            ("can't"), // apostrophe joins letters
            ("e.g"),
            ("var1"),
            ("123,456"), // digits+comma+digits
            ("foo_bar"),
            ("example.com"),
            ("127.0.0.1"),
            ("9090"), // port number
        ];
        assert_eq!(words, expected);
    }

    /// Strategy that yields every code-point from NUL (0) to DEL (127).
    fn ascii_char() -> impl Strategy<Value = char> {
        (0u8..=127).prop_map(|b| b as char)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]
        /// Fast path must equal general path for any ASCII input.
        #[test]
        fn proptest_ascii_matches_unicode_word_indices(
            // Vec<char> → String, length 0‒99
            s in proptest::collection::vec(ascii_char(), 0..100)
                   .prop_map(|v| v.into_iter().collect::<String>())
        ) {
            let fast: Vec<(usize, &str)> = new_ascii_word_bound_indices(&s).collect();
            let uni:  Vec<(usize, &str)> = new_word_bound_indices(&s).collect();

            prop_assert_eq!(fast, uni);
        }

        /// Fast path must equal general path for any ASCII input, forwards and backwards.
        #[test]
        fn proptest_ascii_matches_unicode_word_indices_rev(
            // Vec<char> → String, length 0‒99
            s in proptest::collection::vec(ascii_char(), 0..100)
                   .prop_map(|v| v.into_iter().collect::<String>())
        ) {
            let fast_rev: Vec<(usize, &str)> = new_ascii_word_bound_indices(&s).rev().collect();
            let uni_rev : Vec<(usize, &str)> = new_word_bound_indices(&s).rev().collect();
            prop_assert_eq!(fast_rev, uni_rev);
        }
    }
}
