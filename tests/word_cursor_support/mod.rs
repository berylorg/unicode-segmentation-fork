use unicode_segmentation::{UnicodeSegmentation, WordCursor, WordCursorResult};

pub fn expected_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries: Vec<_> = text
        .split_word_bound_indices()
        .map(|(offset, _)| offset)
        .collect();
    if boundaries.last().copied() != Some(text.len()) {
        boundaries.push(text.len());
    }
    boundaries
}

pub fn forward_boundaries(text: &str, cuts: &[usize]) -> Vec<usize> {
    let mut cursor = WordCursor::new(0, text.len()).unwrap();
    let mut result = vec![0];
    loop {
        let offset = cursor.cur_cursor();
        let end = cuts
            .iter()
            .copied()
            .find(|&cut| cut > offset)
            .unwrap_or(text.len());
        match cursor.next_boundary(&text[offset..end], offset).unwrap() {
            WordCursorResult::Boundary(boundary) => {
                assert_ne!(result.last().copied(), Some(boundary), "text={text:?}");
                result.push(boundary);
            }
            WordCursorResult::NextChunk(requested) => assert_eq!(requested, end),
            WordCursorResult::End => break,
            WordCursorResult::PrevChunk(_) => {
                panic!("forward traversal requested a previous chunk")
            }
            WordCursorResult::PreContext(_) | WordCursorResult::PostContext(_) => {
                panic!("boundary traversal unexpectedly requested initialization context")
            }
        }
    }
    result
}

pub fn reverse_boundaries(text: &str, cuts: &[usize]) -> Vec<usize> {
    let mut cursor = WordCursor::new(text.len(), text.len()).unwrap();
    let mut result = vec![text.len()];
    loop {
        let offset = cursor.cur_cursor();
        let start = cuts
            .iter()
            .copied()
            .rev()
            .find(|&cut| cut < offset)
            .unwrap_or(0);
        match cursor.prev_boundary(&text[start..offset], start).unwrap() {
            WordCursorResult::Boundary(boundary) => {
                assert_ne!(result.last().copied(), Some(boundary), "text={text:?}");
                result.push(boundary);
            }
            WordCursorResult::PrevChunk(requested) => assert_eq!(requested, start),
            WordCursorResult::End => break,
            WordCursorResult::NextChunk(_) => panic!("reverse traversal requested a next chunk"),
            WordCursorResult::PreContext(_) | WordCursorResult::PostContext(_) => {
                panic!("boundary traversal unexpectedly requested initialization context")
            }
        }
    }
    result
}

pub fn previous_scalar_start(text: &str, end: usize) -> usize {
    text[..end]
        .char_indices()
        .next_back()
        .map_or(end, |(offset, _)| offset)
}

pub fn next_scalar_end(text: &str, start: usize) -> usize {
    text[start..]
        .chars()
        .next()
        .map_or(start, |ch| start + ch.len_utf8())
}

pub fn next_from(text: &str, origin: usize) -> WordCursorResult {
    let mut cursor = WordCursor::new(origin, text.len()).unwrap();
    let initial_end = next_scalar_end(text, origin);
    let mut result = cursor
        .next_boundary(&text[origin..initial_end], origin)
        .unwrap();
    loop {
        result = match result {
            WordCursorResult::Boundary(_) | WordCursorResult::End => return result,
            WordCursorResult::PreContext(end) => {
                let start = previous_scalar_start(text, end);
                cursor.next_boundary(&text[start..end], start).unwrap()
            }
            WordCursorResult::NextChunk(start) => {
                let end = next_scalar_end(text, start);
                cursor.next_boundary(&text[start..end], start).unwrap()
            }
            WordCursorResult::PrevChunk(_) | WordCursorResult::PostContext(_) => {
                panic!("forward query requested reverse-direction input")
            }
        };
    }
}

pub fn previous_from(text: &str, origin: usize) -> WordCursorResult {
    let mut cursor = WordCursor::new(origin, text.len()).unwrap();
    let initial_start = previous_scalar_start(text, origin);
    let mut result = cursor
        .prev_boundary(&text[initial_start..origin], initial_start)
        .unwrap();
    loop {
        result = match result {
            WordCursorResult::Boundary(_) | WordCursorResult::End => return result,
            WordCursorResult::PostContext(start) => {
                let end = next_scalar_end(text, start);
                cursor.prev_boundary(&text[start..end], start).unwrap()
            }
            WordCursorResult::PrevChunk(end) => {
                let start = previous_scalar_start(text, end);
                cursor.prev_boundary(&text[start..end], start).unwrap()
            }
            WordCursorResult::NextChunk(_) | WordCursorResult::PreContext(_) => {
                panic!("reverse query requested forward-direction input")
            }
        };
    }
}

fn all_partitions(text: &str) -> Vec<Vec<usize>> {
    let interior: Vec<_> = text
        .char_indices()
        .map(|(offset, _)| offset)
        .filter(|&offset| offset != 0)
        .collect();
    assert!(interior.len() < usize::BITS as usize);
    (0..(1usize << interior.len()))
        .map(|mask| {
            let mut cuts = vec![0];
            cuts.extend(
                interior
                    .iter()
                    .enumerate()
                    .filter_map(|(bit, &cut)| ((mask & (1 << bit)) != 0).then_some(cut)),
            );
            cuts.push(text.len());
            cuts
        })
        .collect()
}

pub fn assert_differential(text: &str, exhaustive: bool) {
    let expected = expected_boundaries(text);
    let partitions = if exhaustive {
        all_partitions(text)
    } else {
        let mut boundaries: Vec<_> = text.char_indices().map(|(offset, _)| offset).collect();
        boundaries.push(text.len());
        vec![vec![0, text.len()], boundaries]
    };

    for cuts in partitions {
        let forward = forward_boundaries(text, &cuts);
        assert_eq!(forward, expected, "cuts={cuts:?}");
        let streamed_segments: Vec<_> = forward
            .windows(2)
            .map(|range| &text[range[0]..range[1]])
            .collect();
        assert_eq!(
            streamed_segments,
            text.split_word_bounds().collect::<Vec<_>>(),
            "cuts={cuts:?}"
        );
        let mut reverse_expected = expected.clone();
        reverse_expected.reverse();
        assert_eq!(
            reverse_boundaries(text, &cuts),
            reverse_expected,
            "cuts={cuts:?}"
        );
    }
}

pub fn assert_arbitrary_offsets(text: &str) {
    let boundaries = expected_boundaries(text);
    for origin in text
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(core::iter::once(text.len()))
    {
        let expected_next = boundaries
            .iter()
            .copied()
            .find(|&boundary| boundary > origin)
            .map_or(WordCursorResult::End, WordCursorResult::Boundary);
        let expected_previous = boundaries
            .iter()
            .copied()
            .rev()
            .find(|&boundary| boundary < origin)
            .map_or(WordCursorResult::End, WordCursorResult::Boundary);
        assert_eq!(
            next_from(text, origin),
            expected_next,
            "forward text={text:?}, origin={origin}"
        );
        assert_eq!(
            previous_from(text, origin),
            expected_previous,
            "reverse text={text:?}, origin={origin}"
        );
    }
}
