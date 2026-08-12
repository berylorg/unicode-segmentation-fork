# Goals

Preserve the complete public behavior of `unicode-segmentation` 1.13.2 while adding an
application-neutral way to traverse exact Unicode word boundaries over bounded, non-contiguous
UTF-8 chunks.

Keep streaming traversal memory bounded independently of document length, segment length, and
chunk count.

# Decisions

The fork retains every existing public API and the Unicode word-break tables and policy from the
exact 1.13.2 base.

The additive public word cursor reports global byte offsets and supports both forward and reverse
boundary traversal. A caller supplies one borrowed UTF-8 chunk at a time at the exact requested
offset. The cursor never retains a supplied chunk.

Chunk requests and contract violations are typed results. A request identifies the exact next
chunk start or previous chunk end. Errors distinguish invalid document offsets, arithmetic or
document-range violations, gaps, overlaps, stale chunks, and a direction change while traversal is
incomplete. Document start and end are resolved explicitly and are never guessed from a chunk
edge.

Traversal from an arbitrary scalar offset establishes exact state through typed bounded context
requests. Forward traversal requests adjacent pre-context back to document start and then replays
bounded chunks toward the starting offset. Reverse traversal requests adjacent post-context to
document end and then replays bounded chunks toward the starting offset. This protocol may visit
many chunks but retains none of them.

Continuation state is a fixed-size Unicode word-break state machine. It may retain scalar
categories, counters, and global offsets, but it does not retain source text or allocate storage
that grows with input.

Streaming boundaries use the same generated word-category and extended-pictographic tables as the
contiguous iterators. Differential tests define equivalence with `split_word_bound_indices` and
`split_word_bounds` in both directions across bounded chunk partitions.
