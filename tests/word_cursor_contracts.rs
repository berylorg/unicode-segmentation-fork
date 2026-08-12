use unicode_segmentation::{WordCursor, WordCursorError, WordCursorResult};

#[test]
fn validates_chunk_and_direction_contracts() {
    assert_eq!(
        WordCursor::new(4, 3).unwrap_err(),
        WordCursorError::InvalidOffset
    );

    let mut forward = WordCursor::new(0, 4).unwrap();
    assert_eq!(
        forward.next_boundary("", 0),
        Err(WordCursorError::EmptyChunk)
    );
    assert_eq!(
        forward.next_boundary("a", 1),
        Err(WordCursorError::Gap {
            expected: 0,
            actual: 1,
        })
    );
    assert_eq!(
        forward.next_boundary("abcde", 0),
        Err(WordCursorError::ChunkOutOfBounds)
    );
    assert_eq!(
        forward.next_boundary("ab", 0).unwrap(),
        WordCursorResult::NextChunk(2)
    );
    assert_eq!(
        forward.next_boundary("ab", 0),
        Err(WordCursorError::Overlap {
            expected: 2,
            actual: 0,
        })
    );
    assert_eq!(
        forward.prev_boundary("ab", 0),
        Err(WordCursorError::WrongDirection)
    );

    let mut reverse = WordCursor::new(4, 4).unwrap();
    assert_eq!(
        reverse.prev_boundary("", 4),
        Err(WordCursorError::EmptyChunk)
    );
    assert_eq!(
        reverse.prev_boundary("a", 1),
        Err(WordCursorError::Gap {
            expected: 4,
            actual: 2,
        })
    );
    assert_eq!(
        reverse.prev_boundary("abc", 2),
        Err(WordCursorError::ChunkOutOfBounds)
    );
    assert_eq!(
        reverse.prev_boundary("cd", 2).unwrap(),
        WordCursorResult::PrevChunk(2)
    );
    assert_eq!(
        reverse.prev_boundary("bcd", 1),
        Err(WordCursorError::Overlap {
            expected: 2,
            actual: 4,
        })
    );
    assert_eq!(
        reverse.next_boundary("cd", 2),
        Err(WordCursorError::WrongDirection)
    );

    let mut pre_context = WordCursor::new(1, 3).unwrap();
    assert_eq!(
        pre_context.next_boundary("b", 1),
        Ok(WordCursorResult::PreContext(1))
    );
    assert_eq!(
        pre_context.next_boundary("", 1),
        Err(WordCursorError::EmptyChunk)
    );
    assert_eq!(
        pre_context.next_boundary("", 0),
        Err(WordCursorError::Gap {
            expected: 1,
            actual: 0,
        })
    );
    assert_eq!(
        pre_context.next_boundary("ab", 0),
        Err(WordCursorError::Overlap {
            expected: 1,
            actual: 2,
        })
    );

    let mut post_context = WordCursor::new(1, 3).unwrap();
    assert_eq!(
        post_context.prev_boundary("a", 0),
        Ok(WordCursorResult::PostContext(1))
    );
    assert_eq!(
        post_context.prev_boundary("", 1),
        Err(WordCursorError::EmptyChunk)
    );
    assert_eq!(
        post_context.prev_boundary("", 2),
        Err(WordCursorError::Gap {
            expected: 1,
            actual: 2,
        })
    );
    assert_eq!(
        post_context.prev_boundary("a", 0),
        Err(WordCursorError::Overlap {
            expected: 1,
            actual: 0,
        })
    );

    assert_eq!(
        WordCursor::new(0, 0).unwrap().next_boundary("", 0),
        Ok(WordCursorResult::End)
    );
    assert_eq!(
        WordCursor::new(0, 0).unwrap().prev_boundary("", 0),
        Ok(WordCursorResult::End)
    );
}

#[test]
fn validates_contracts_at_document_edges() {
    let mut forward_end = WordCursor::new(3, 3).unwrap();
    assert_eq!(
        forward_end.next_boundary("", 2),
        Err(WordCursorError::Overlap {
            expected: 3,
            actual: 2,
        })
    );
    assert_eq!(
        forward_end.next_boundary("x", 3),
        Err(WordCursorError::ChunkOutOfBounds)
    );
    assert_eq!(
        forward_end.next_boundary("", usize::MAX),
        Err(WordCursorError::ChunkOutOfBounds)
    );
    assert_eq!(forward_end.next_boundary("", 3), Ok(WordCursorResult::End));

    let mut reverse_start = WordCursor::new(0, 3).unwrap();
    assert_eq!(
        reverse_start.prev_boundary("", 1),
        Err(WordCursorError::Overlap {
            expected: 0,
            actual: 1,
        })
    );
    assert_eq!(
        reverse_start.prev_boundary("x", 0),
        Err(WordCursorError::Overlap {
            expected: 0,
            actual: 1,
        })
    );
    assert_eq!(
        reverse_start.prev_boundary("x", usize::MAX),
        Err(WordCursorError::ChunkOutOfBounds)
    );
    assert_eq!(
        reverse_start.prev_boundary("", 0),
        Ok(WordCursorResult::End)
    );

    assert_eq!(
        WordCursor::new(0, 1).unwrap().next_boundary("a", 0),
        Ok(WordCursorResult::Boundary(1))
    );
    assert_eq!(
        WordCursor::new(1, 1).unwrap().prev_boundary("a", 0),
        Ok(WordCursorResult::Boundary(0))
    );

    let mut at_start_wrong_direction = WordCursor::new(1, 2).unwrap();
    assert_eq!(
        at_start_wrong_direction.next_boundary("b", 1),
        Ok(WordCursorResult::PreContext(1))
    );
    assert_eq!(
        at_start_wrong_direction.next_boundary("a", 0),
        Ok(WordCursorResult::NextChunk(0))
    );
    assert_eq!(
        at_start_wrong_direction.prev_boundary("", 0),
        Err(WordCursorError::WrongDirection)
    );

    let mut at_end_wrong_direction = WordCursor::new(1, 2).unwrap();
    assert_eq!(
        at_end_wrong_direction.prev_boundary("a", 0),
        Ok(WordCursorResult::PostContext(1))
    );
    assert_eq!(
        at_end_wrong_direction.prev_boundary("b", 1),
        Ok(WordCursorResult::PrevChunk(2))
    );
    assert_eq!(
        at_end_wrong_direction.next_boundary("", 2),
        Err(WordCursorError::WrongDirection)
    );
}
