use core::mem::{size_of, size_of_val};

use unicode_segmentation::{WordCursor, WordCursorResult};

#[allow(dead_code)]
mod word_cursor_support;

use word_cursor_support::{expected_boundaries, next_scalar_end, previous_scalar_start};

#[test]
fn pre_context_can_span_many_temporary_chunks() {
    let text = "a\u{0301}".repeat(512) + "!";
    let origin = text.char_indices().nth(900).unwrap().0;
    let mut cursor = WordCursor::new(origin, text.len()).unwrap();
    let initial_end = next_scalar_end(&text, origin);
    let mut result = cursor
        .next_boundary(&text[origin..initial_end], origin)
        .unwrap();
    let mut context_chunks = 0;

    while let WordCursorResult::PreContext(end) = result {
        let start = previous_scalar_start(&text, end);
        let temporary = text[start..end].to_owned();
        result = cursor.next_boundary(&temporary, start).unwrap();
        context_chunks += 1;
        assert_eq!(size_of_val(&cursor), size_of::<WordCursor>());
    }
    assert!(context_chunks > 800);
    assert_eq!(result, WordCursorResult::NextChunk(0));

    loop {
        result = match result {
            WordCursorResult::Boundary(boundary) => {
                assert_eq!(
                    Some(boundary),
                    expected_boundaries(&text)
                        .into_iter()
                        .find(|&candidate| candidate > origin)
                );
                break;
            }
            WordCursorResult::NextChunk(start) => {
                let end = next_scalar_end(&text, start);
                let temporary = text[start..end].to_owned();
                cursor.next_boundary(&temporary, start).unwrap()
            }
            other => panic!("unexpected replay result: {:?}", other),
        };
    }
}

#[test]
fn cursor_is_fixed_size_and_does_not_retain_chunks() {
    assert!(size_of::<WordCursor>() <= 256);
    let mut cursor = WordCursor::new(0, 4).unwrap();
    {
        let temporary = String::from("ab");
        assert_eq!(
            cursor.next_boundary(&temporary, 0).unwrap(),
            WordCursorResult::NextChunk(2)
        );
    }
    {
        let temporary = String::from("cd");
        assert_eq!(
            cursor.next_boundary(&temporary, 2).unwrap(),
            WordCursorResult::Boundary(4)
        );
    }
    assert_eq!(size_of_val(&cursor), size_of::<WordCursor>());
}
