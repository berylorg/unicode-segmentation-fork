use core::mem::{needs_drop, size_of, size_of_val};

use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete, UnicodeSegmentation};

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "testdata/mod.rs"]
mod testdata;

#[derive(Clone, Copy, Debug)]
struct Page {
    start: usize,
    end: usize,
}

fn scalar_offsets(text: &str) -> Vec<usize> {
    text.char_indices()
        .map(|(offset, _)| offset)
        .chain(core::iter::once(text.len()))
        .collect()
}

fn pages_from_mask(text: &str, mask: usize) -> Vec<Page> {
    let offsets = scalar_offsets(text);
    let mut boundaries = vec![0];
    for (index, &offset) in offsets[1..offsets.len() - 1].iter().enumerate() {
        if mask & (1 << index) != 0 {
            boundaries.push(offset);
        }
    }
    boundaries.push(text.len());
    boundaries
        .windows(2)
        .map(|window| Page {
            start: window[0],
            end: window[1],
        })
        .collect()
}

fn scalar_pages(text: &str) -> Vec<Page> {
    scalar_offsets(text)
        .windows(2)
        .map(|window| Page {
            start: window[0],
            end: window[1],
        })
        .collect()
}

fn expected_boundaries(text: &str, is_extended: bool) -> Vec<usize> {
    text.grapheme_indices(is_extended)
        .map(|(offset, _)| offset)
        .chain(core::iter::once(text.len()))
        .collect()
}

fn forward_page(pages: &[Page], offset: usize, len: usize) -> usize {
    if offset == len {
        return pages.len() - 1;
    }
    pages
        .iter()
        .position(|page| page.start <= offset && offset < page.end)
        .unwrap()
}

fn reverse_page(pages: &[Page], offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    pages
        .iter()
        .position(|page| page.start < offset && offset <= page.end)
        .unwrap()
}

fn context_page(pages: &[Page], end: usize) -> usize {
    pages.iter().rposition(|page| page.end == end).unwrap()
}

fn provide_requested_context(cursor: &mut GraphemeCursor, text: &str, pages: &[Page], end: usize) {
    let page = pages[context_page(pages, end)];
    cursor.provide_context(&text[page.start..page.end], page.start);
}

fn forward_boundaries(text: &str, is_extended: bool, pages: &[Page]) -> Vec<usize> {
    let mut cursor = GraphemeCursor::new(0, text.len(), is_extended);
    let mut active = 0;
    let mut boundaries = vec![0];
    loop {
        let page = pages[active];
        match cursor.next_boundary(&text[page.start..page.end], page.start) {
            Ok(Some(boundary)) => {
                boundaries.push(boundary);
                active = forward_page(pages, boundary, text.len());
            }
            Ok(None) => return boundaries,
            Err(GraphemeIncomplete::NextChunk) => active += 1,
            Err(GraphemeIncomplete::PreContext(end)) => {
                provide_requested_context(&mut cursor, text, pages, end);
            }
            Err(other) => panic!("unexpected forward result for {:?}: {:?}", text, other),
        }
    }
}

fn reverse_boundaries(text: &str, is_extended: bool, pages: &[Page]) -> Vec<usize> {
    let mut cursor = GraphemeCursor::new(text.len(), text.len(), is_extended);
    let mut active = pages.len() - 1;
    let mut boundaries = Vec::new();
    loop {
        let page = pages[active];
        match cursor.prev_boundary(&text[page.start..page.end], page.start) {
            Ok(Some(boundary)) => {
                boundaries.push(boundary);
                active = reverse_page(pages, boundary);
            }
            Ok(None) => return boundaries,
            Err(GraphemeIncomplete::PrevChunk) => active -= 1,
            Err(GraphemeIncomplete::PreContext(end)) => {
                provide_requested_context(&mut cursor, text, pages, end);
            }
            Err(other) => panic!("unexpected reverse result for {:?}: {:?}", text, other),
        }
    }
}

fn boundary_at(text: &str, offset: usize, is_extended: bool, pages: &[Page]) -> bool {
    let mut cursor = GraphemeCursor::new(offset, text.len(), is_extended);
    let active = forward_page(pages, offset, text.len());
    loop {
        let page = pages[active];
        match cursor.is_boundary(&text[page.start..page.end], page.start) {
            Ok(result) => return result,
            Err(GraphemeIncomplete::PreContext(end)) => {
                provide_requested_context(&mut cursor, text, pages, end);
            }
            Err(other) => panic!(
                "unexpected boundary result at {} for {:?}: {:?}",
                offset, text, other
            ),
        }
    }
}

fn next_from(text: &str, offset: usize, is_extended: bool, pages: &[Page]) -> Option<usize> {
    let mut cursor = GraphemeCursor::new(offset, text.len(), is_extended);
    let mut active = forward_page(pages, offset, text.len());
    loop {
        let page = pages[active];
        match cursor.next_boundary(&text[page.start..page.end], page.start) {
            Ok(result) => return result,
            Err(GraphemeIncomplete::NextChunk) => active += 1,
            Err(GraphemeIncomplete::PreContext(end)) => {
                provide_requested_context(&mut cursor, text, pages, end);
            }
            Err(other) => panic!(
                "unexpected next result from {} for {:?}: {:?}",
                offset, text, other
            ),
        }
    }
}

fn previous_from(text: &str, offset: usize, is_extended: bool, pages: &[Page]) -> Option<usize> {
    let mut cursor = GraphemeCursor::new(offset, text.len(), is_extended);
    let mut active = reverse_page(pages, offset);
    loop {
        let page = pages[active];
        match cursor.prev_boundary(&text[page.start..page.end], page.start) {
            Ok(result) => return result,
            Err(GraphemeIncomplete::PrevChunk) => active -= 1,
            Err(GraphemeIncomplete::PreContext(end)) => {
                provide_requested_context(&mut cursor, text, pages, end);
            }
            Err(other) => panic!(
                "unexpected previous result from {} for {:?}: {:?}",
                offset, text, other
            ),
        }
    }
}

fn assert_partition_matches_contiguous(text: &str, is_extended: bool, pages: &[Page]) {
    let expected = expected_boundaries(text, is_extended);
    assert_eq!(
        forward_boundaries(text, is_extended, pages),
        expected,
        "forward {text:?}, extended={is_extended}, pages={pages:?}"
    );

    let mut expected_reverse = expected[..expected.len() - 1].to_vec();
    expected_reverse.reverse();
    assert_eq!(
        reverse_boundaries(text, is_extended, pages),
        expected_reverse,
        "reverse {text:?}, extended={is_extended}, pages={pages:?}"
    );

    for offset in scalar_offsets(text) {
        let expected_next = expected.iter().copied().find(|boundary| *boundary > offset);
        let expected_previous = expected
            .iter()
            .rev()
            .copied()
            .find(|boundary| *boundary < offset);
        assert_eq!(
            boundary_at(text, offset, is_extended, pages),
            expected.contains(&offset),
            "is_boundary at {offset} for {text:?}, extended={is_extended}, pages={pages:?}"
        );
        assert_eq!(
            next_from(text, offset, is_extended, pages),
            expected_next,
            "next_boundary from {offset} for {text:?}, extended={is_extended}, pages={pages:?}"
        );
        assert_eq!(
            previous_from(text, offset, is_extended, pages),
            expected_previous,
            "prev_boundary from {offset} for {text:?}, extended={is_extended}, pages={pages:?}"
        );
    }
}

fn assert_all_partitions_match(text: &str) {
    let internal_boundaries = text.chars().count() - 1;
    for mask in 0..(1 << internal_boundaries) {
        let pages = pages_from_mask(text, mask);
        for is_extended in [false, true] {
            assert_partition_matches_contiguous(text, is_extended, &pages);
        }
    }
}

#[test]
fn gb11_retains_the_post_zwj_phase_across_scalar_context_pages() {
    let text = "\u{1F468}\u{0308}\u{0301}\u{200D}\u{1F469}";
    let offsets = scalar_offsets(text);
    let boundary = offsets[offsets.len() - 2];
    let mut cursor = GraphemeCursor::new(boundary, text.len(), true);

    assert_eq!(
        cursor.is_boundary(&text[boundary..], boundary),
        Err(GraphemeIncomplete::PreContext(boundary))
    );
    for window in offsets[..offsets.len() - 1].windows(2).rev() {
        cursor.provide_context(&text[window[0]..window[1]], window[0]);
        match cursor.is_boundary(&text[boundary..], boundary) {
            Err(GraphemeIncomplete::PreContext(next)) => assert_eq!(next, window[0]),
            Ok(false) => {
                assert_eq!(window[0], 0);
                return;
            }
            other => panic!("GB11 resolved incorrectly after {:?}: {:?}", window, other),
        }
    }
    panic!("GB11 did not resolve after complete pre-context");
}

#[test]
fn exhaustive_scalar_boundary_partitions_match_contiguous_segmentation() {
    let representatives = [
        "\u{1F468}\u{0308}\u{200D}\u{1F469}",
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
        "\u{1F1E6}\u{1F1E7}\u{1F1E8}\u{1F1E9}",
        "\u{0915}\u{094D}\u{0300}\u{0915}",
        "\u{0600}\u{0600}a",
        "\r\n\u{0000}",
    ];

    for text in representatives {
        assert_all_partitions_match(text);
    }
}

#[test]
fn unicode_17_grapheme_break_corpus_matches_with_scalar_pages() {
    use testdata::{TEST_DIFF, TEST_SAME};

    for &(text, _) in TEST_SAME {
        let pages = scalar_pages(text);
        for is_extended in [false, true] {
            assert_partition_matches_contiguous(text, is_extended, &pages);
        }
    }
    for &(text, _, _) in TEST_DIFF {
        let pages = scalar_pages(text);
        for is_extended in [false, true] {
            assert_partition_matches_contiguous(text, is_extended, &pages);
        }
    }
}

#[test]
fn adversarial_rule_families_match_with_scalar_pages() {
    let mut cases = Vec::new();
    for length in [1, 8, 32, 128] {
        cases.push(format!(
            "x\u{1F468}{}\u{200D}\u{1F469}y",
            "\u{0308}".repeat(length)
        ));
        cases.push(format!("x{}y", "\u{1F468}\u{200D}".repeat(length)));
        cases.push(format!("x{}y", "\u{1F1E6}".repeat(length * 2 + 1)));
        cases.push(format!("x{}ay", "\u{0600}".repeat(length)));
        cases.push(format!(
            "x\u{0915}{}\u{0915}y",
            "\u{094D}\u{0300}".repeat(length)
        ));
    }
    cases.extend([
        "\r\n\u{0000}a\r\nb".to_owned(),
        "\u{0000}\u{0308}\r\n\u{0001}".to_owned(),
    ]);

    for text in &cases {
        let pages = scalar_pages(text);
        for is_extended in [false, true] {
            assert_partition_matches_contiguous(text, is_extended, &pages);
        }
    }
}

#[test]
fn deterministic_random_offsets_and_partitions_match_contiguous_boundaries() {
    let texts = [
        "a\u{0308}\u{0301}b",
        "\u{1F468}\u{0308}\u{200D}\u{1F469}z",
        "\u{1F1E6}\u{1F1E7}\u{1F1E8}\u{1F1E9}\u{1F1EA}",
        "\u{0915}\u{094D}\u{0300}\u{094D}\u{0915}",
        "\u{0600}\u{0600}a\r\n",
    ];
    let mut random = 0xD1B5_4A32_D192_ED03_u64;

    for _ in 0..256 {
        random = random
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let text = texts[random as usize % texts.len()];
        let offsets = scalar_offsets(text);
        let internal = offsets.len() - 2;
        let mask = (random.rotate_left(17) as usize) & ((1 << internal) - 1);
        let pages = pages_from_mask(text, mask);
        let offset = offsets[random.rotate_left(31) as usize % offsets.len()];
        for is_extended in [false, true] {
            let expected = expected_boundaries(text, is_extended);
            assert_eq!(
                boundary_at(text, offset, is_extended, &pages),
                expected.contains(&offset),
                "random offset {offset} for {text:?}, extended={is_extended}, pages={pages:?}"
            );
        }
    }
}

#[test]
fn cursor_state_size_is_independent_of_context_length() {
    assert!(size_of::<GraphemeCursor>() <= 128);
    assert!(!needs_drop::<GraphemeCursor>());

    for length in [1, 8, 64, 256] {
        let text = format!("\u{1F468}{}\u{200D}\u{1F469}", "\u{0308}".repeat(length));
        let pages = scalar_pages(&text);
        let mut cursor = GraphemeCursor::new(text.len(), text.len(), true);
        assert_eq!(size_of_val(&cursor), size_of::<GraphemeCursor>());
        while cursor.prev_boundary(&text, 0).unwrap().is_some() {
            assert_eq!(size_of_val(&cursor), size_of::<GraphemeCursor>());
        }
        assert_partition_matches_contiguous(&text, true, &pages);
    }
}

#[test]
fn document_edges_and_invalid_offsets_keep_existing_results() {
    assert_eq!(
        GraphemeCursor::new(0, 0, true).next_boundary("", 0),
        Ok(None)
    );
    assert_eq!(
        GraphemeCursor::new(0, 0, true).prev_boundary("", 0),
        Ok(None)
    );

    let mut cursor = GraphemeCursor::new(1, 3, true);
    assert_eq!(
        cursor.is_boundary("c", 2),
        Err(GraphemeIncomplete::InvalidOffset)
    );
}

#[test]
#[should_panic]
fn non_adjacent_pre_context_keeps_the_documented_assertion() {
    let text = "\u{1F468}\u{200D}\u{1F469}";
    let boundary = "\u{1F468}\u{200D}".len();
    let mut cursor = GraphemeCursor::new(boundary, text.len(), true);
    assert_eq!(
        cursor.is_boundary(&text[boundary..], boundary),
        Err(GraphemeIncomplete::PreContext(boundary))
    );
    cursor.provide_context("x", 0);
}
