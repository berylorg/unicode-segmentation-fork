mod word_cursor_support;

use unicode_segmentation::{WordCursor, WordCursorResult};
use word_cursor_support::{
    assert_arbitrary_offsets, assert_differential, expected_boundaries, forward_boundaries,
    next_from, previous_from, reverse_boundaries,
};

#[path = "testdata/mod.rs"]
#[allow(dead_code)]
#[rustfmt::skip]
mod testdata;

#[test]
fn differential_all_partitions_for_short_rule_cases() {
    for text in [
        "",
        "abc",
        "a b",
        "a\r\nb",
        "a.b",
        "3,14",
        "a\u{0301}b",
        "a\u{200d}\u{1f469}",
        "\u{1f1e6}\u{1f1e7}\u{1f1e8}",
        "\u{05d0}'\u{05d1}",
        "\u{05d0}\"\u{05d1}",
    ] {
        assert_differential(text, true);
    }
}

#[test]
fn differential_representative_unicode_and_long_segment() {
    for text in [
        "Hello, 世界 123.45!",
        "カタカナ_42 can't",
        "a\u{00ad}\u{0301}b\u{200d}\u{1f469}!",
        "\r\n\u{0085}\u{2028}  \t",
        "\u{1f1e6}\u{0301}\u{1f1e7}\u{1f1e8}\u{1f1e9}",
    ] {
        assert_differential(text, false);
    }

    let long = "a\u{0301}".repeat(2048);
    assert_differential(&long, false);
}

#[test]
fn differential_official_word_break_vectors_at_every_scalar_edge() {
    for &(text, expected_segments) in testdata::TEST_WORD {
        let mut cuts: Vec<_> = text.char_indices().map(|(offset, _)| offset).collect();
        cuts.push(text.len());
        let expected: Vec<_> = expected_segments
            .iter()
            .scan(0, |offset, segment| {
                let result = *offset;
                *offset += segment.len();
                Some(result)
            })
            .chain(core::iter::once(text.len()))
            .collect();
        assert_eq!(expected_boundaries(text), expected);
        assert_eq!(forward_boundaries(text, &cuts), expected, "text={text:?}");
        let mut reverse_expected = expected;
        reverse_expected.reverse();
        assert_eq!(
            reverse_boundaries(text, &cuts),
            reverse_expected,
            "text={text:?}"
        );
    }
}

#[test]
fn differential_official_word_break_vectors_from_every_scalar_offset() {
    for &(text, _) in testdata::TEST_WORD {
        assert_arbitrary_offsets(text);
    }
}

#[test]
fn differential_from_every_scalar_offset() {
    for text in [
        "a.b",
        "3,14",
        "\u{05d0}\"\u{05d1} \u{05d0}'\u{05d1}",
        "a\u{0301}\u{00ad}b",
        "x\u{200d}\u{1f469}y",
        "\r\nx\u{0085}y\u{2028}z",
        "\u{30ab}\u{30bf}\u{30ab}\u{30ca}_42",
        "\u{1f1e6}\u{1f1e7}\u{1f1e8}",
    ] {
        assert_arbitrary_offsets(text);
    }
}

#[test]
fn failed_connector_lookahead_from_every_scalar_offset() {
    for text in [
        "a:.a",
        "a\u{0301}:\u{0301}.a",
        "a\u{2060}:\u{2060}.a",
        "a.:a",
        "3,a3",
        "3\u{0301},\u{0301}a3",
        "3\u{2060}'\u{2060}a3",
        "\u{05d0}\"a\u{05d0}",
        "\u{05d0}\u{0301}\"\u{0301}a\u{05d0}",
        "\u{05d0}\u{2060}\"\u{2060}a\u{05d0}",
    ] {
        assert_arbitrary_offsets(text);
    }
}

#[test]
fn arbitrary_offset_regressions() {
    assert_eq!(next_from("a.b", 1), WordCursorResult::Boundary(3));
    assert_eq!(previous_from("a.b", 2), WordCursorResult::Boundary(0));
    assert_eq!(next_from("a:.a", 1), WordCursorResult::Boundary(2));
    assert_eq!(previous_from("a:.a", 3), WordCursorResult::Boundary(2));

    let regional = "\u{1f1e6}\u{1f1e7}\u{1f1e8}";
    assert_eq!(next_from(regional, 0), WordCursorResult::Boundary(8));
    assert_eq!(next_from(regional, 4), WordCursorResult::Boundary(8));
    assert_eq!(next_from(regional, 8), WordCursorResult::Boundary(12));
    assert_eq!(previous_from(regional, 4), WordCursorResult::Boundary(0));
    assert_eq!(previous_from(regional, 8), WordCursorResult::Boundary(0));

    let mut reset = WordCursor::new(0, 3).unwrap();
    reset.set_cursor(1).unwrap();
    assert_eq!(
        reset.next_boundary(".", 1),
        Ok(WordCursorResult::PreContext(1))
    );
    assert_eq!(
        reset.next_boundary("a", 0),
        Ok(WordCursorResult::NextChunk(0))
    );
    assert_eq!(
        reset.next_boundary("a.b", 0),
        Ok(WordCursorResult::Boundary(3))
    );

    reset.set_cursor(2).unwrap();
    assert_eq!(
        reset.prev_boundary(".", 1),
        Ok(WordCursorResult::PostContext(2))
    );
    assert_eq!(
        reset.prev_boundary("b", 2),
        Ok(WordCursorResult::PrevChunk(3))
    );
    assert_eq!(
        reset.prev_boundary("a.b", 0),
        Ok(WordCursorResult::Boundary(0))
    );
}
