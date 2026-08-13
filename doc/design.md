# Goals

Preserve the complete public behavior of `unicode-segmentation` 1.13.2 while providing
application-neutral traversal of exact Unicode word and grapheme boundaries over bounded,
non-contiguous UTF-8 chunks.

Keep streaming traversal memory bounded independently of document length, segment length, and
chunk count.

# Decisions

The fork retains every existing public API and the Unicode word-break tables and policy from the
exact 1.13.2 base.

The existing public `GraphemeCursor` remains source-compatible and produces the same extended and
legacy grapheme boundaries as the contiguous APIs. Its non-contiguous pre-context state is
partition-invariant: supplying the same valid context as one chunk or any sequence of exact
adjacent chunks produces the same boundary decision.

Each grapheme rule with reverse pre-context retains sufficient fixed substate across chunk
deliveries. GB11 distinguishes searching for its required ZWJ from scanning the preceding
`Extend*` sequence for `Extended_Pictographic`; it does not require a ZWJ again after a prior chunk
has consumed it. GB9c and regional-indicator context likewise preserve their rule progress across
arbitrary valid partitions. The cursor retains no supplied chunk or grapheme text.

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

Continuation state is a fixed-size Unicode word- or grapheme-break state machine. It may retain
scalar categories, rule substates, counters, and global offsets, but it does not retain source text
or allocate storage that grows with input.

Streaming boundaries use the same generated word-category and extended-pictographic tables as the
contiguous iterators. Differential tests define equivalence with `split_word_bound_indices` and
`split_word_bounds` in both directions across bounded chunk partitions.

Grapheme differential tests define equivalence with the contiguous grapheme APIs and the resolved
Unicode grapheme-break corpus for forward, reverse, random-offset, and boundary-query traversal.
They partition context at every scalar boundary, including one-scalar chunks and adversarially long
combining, emoji-ZWJ, regional-indicator, Prepend, and Indic-conjunct sequences. Increasing a
grapheme's length may increase supplied chunks and total work, but never retained cursor size or
retained source bytes.
