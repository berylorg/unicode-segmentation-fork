#[derive(Clone, Copy, Debug)]
struct ForwardState {
    significant: Option<Significant>,
    significant_offset: usize,
    raw_previous: Option<WordCat>,
    skipped: bool,
    ri_odd: bool,
    pending: Option<ForwardPending>,
}

#[derive(Clone, Copy, Debug)]
struct ForwardPending {
    connector: WordCat,
    connector_offset: usize,
    left: WordCat,
}

impl ForwardState {
    fn new(segment_start: usize) -> Self {
        Self {
            significant: None,
            significant_offset: segment_start,
            raw_previous: None,
            skipped: false,
            ri_odd: false,
            pending: None,
        }
    }

    fn push(&mut self, ch: char, at: usize) -> Option<usize> {
        let category = wd::word_category(ch).2;
        let pictographic = is_extended_pictographic(ch);

        if is_ignored(category) && self.significant.is_some() {
            if matches!(self.significant, Some(Significant::Cat(previous)) if is_newline(previous))
            {
                return Some(at);
            }
            self.raw_previous = Some(category);
            self.skipped = true;
            return None;
        }

        if self.significant.is_none() {
            self.significant = Some(Significant::Cat(category));
            self.significant_offset = at;
            self.raw_previous = Some(category);
            self.ri_odd = category == wd::WC_Regional_Indicator;
            return None;
        }

        if self.raw_previous == Some(wd::WC_ZWJ) && pictographic {
            self.significant = Some(Significant::Emoji);
            self.significant_offset = at;
            self.raw_previous = Some(category);
            self.skipped = false;
            self.pending = None;
            self.ri_odd = false;
            return None;
        }

        let previous = self.significant.unwrap();
        if let Some(pending) = self.pending {
            if connector_matches(pending.left, pending.connector, category) {
                self.pending = None;
                self.significant = Some(Significant::Cat(category));
                self.significant_offset = at;
                self.raw_previous = Some(category);
                self.skipped = false;
                self.ri_odd = category == wd::WC_Regional_Indicator;
                return None;
            }
            return Some(pending.connector_offset);
        }

        let previous_category = match previous {
            Significant::Emoji => return Some(at),
            Significant::Cat(category) => category,
        };

        if is_hard_break(previous_category, category) {
            return Some(at);
        }
        if previous_category == wd::WC_CR && category == wd::WC_LF {
            self.update_significant(category, at);
            return None;
        }
        if previous_category == wd::WC_WSegSpace && category == wd::WC_WSegSpace {
            if !self.skipped {
                self.update_significant(category, at);
                return None;
            }
            return Some(at);
        }
        if previous_category == wd::WC_Hebrew_Letter && category == wd::WC_Single_Quote {
            self.raw_previous = Some(category);
            self.skipped = false;
            return None;
        }
        if pair_matches(previous_category, category) {
            self.update_significant(category, at);
            return None;
        }
        if is_connector_after(previous_category, category) {
            self.pending = Some(ForwardPending {
                connector: category,
                connector_offset: at,
                left: previous_category,
            });
            self.raw_previous = Some(category);
            self.skipped = false;
            return None;
        }
        if previous_category == wd::WC_Regional_Indicator && category == wd::WC_Regional_Indicator {
            if self.ri_odd {
                self.ri_odd = false;
                self.update_significant(category, at);
                return None;
            }
            return Some(at);
        }
        Some(at)
    }

    fn update_significant(&mut self, category: WordCat, at: usize) {
        self.significant = Some(Significant::Cat(category));
        self.significant_offset = at;
        self.raw_previous = Some(category);
        self.skipped = false;
        if category != wd::WC_Regional_Indicator {
            self.ri_odd = false;
        }
    }

    fn finish(&self, document_end: usize) -> usize {
        self.pending
            .map_or(document_end, |pending| pending.connector_offset)
    }
}

fn is_ignored(category: WordCat) -> bool {
    matches!(category, wd::WC_Extend | wd::WC_Format | wd::WC_ZWJ)
}

fn is_newline(category: WordCat) -> bool {
    matches!(category, wd::WC_CR | wd::WC_LF | wd::WC_Newline)
}

fn is_hard_break(left: WordCat, right: WordCat) -> bool {
    (is_newline(left) || is_newline(right)) && !(left == wd::WC_CR && right == wd::WC_LF)
}

fn is_ahletter(category: WordCat) -> bool {
    matches!(category, wd::WC_ALetter | wd::WC_Hebrew_Letter)
}

fn is_wordish(category: WordCat) -> bool {
    is_ahletter(category)
        || matches!(
            category,
            wd::WC_Numeric | wd::WC_Katakana | wd::WC_ExtendNumLet
        )
}

fn pair_matches(left: WordCat, right: WordCat) -> bool {
    (is_ahletter(left) && is_ahletter(right))
        || (left == wd::WC_Hebrew_Letter && right == wd::WC_Single_Quote)
        || (left == wd::WC_Numeric && right == wd::WC_Numeric)
        || (is_ahletter(left) && right == wd::WC_Numeric)
        || (left == wd::WC_Numeric && is_ahletter(right))
        || (left == wd::WC_Katakana && right == wd::WC_Katakana)
        || (is_wordish(left) && right == wd::WC_ExtendNumLet)
        || (left == wd::WC_ExtendNumLet && is_wordish(right))
}

fn is_connector_after(left: WordCat, connector: WordCat) -> bool {
    (is_ahletter(left)
        && matches!(
            connector,
            wd::WC_MidLetter | wd::WC_MidNumLet | wd::WC_Single_Quote
        ))
        || (left == wd::WC_Hebrew_Letter && connector == wd::WC_Double_Quote)
        || (left == wd::WC_Numeric
            && matches!(
                connector,
                wd::WC_MidNum | wd::WC_MidNumLet | wd::WC_Single_Quote
            ))
}

fn connector_matches(left: WordCat, connector: WordCat, right: WordCat) -> bool {
    (is_ahletter(left)
        && matches!(
            connector,
            wd::WC_MidLetter | wd::WC_MidNumLet | wd::WC_Single_Quote
        )
        && is_ahletter(right))
        || (left == wd::WC_Hebrew_Letter
            && connector == wd::WC_Double_Quote
            && right == wd::WC_Hebrew_Letter)
        || (left == wd::WC_Numeric
            && matches!(
                connector,
                wd::WC_MidNum | wd::WC_MidNumLet | wd::WC_Single_Quote
            )
            && right == wd::WC_Numeric)
}

fn is_extended_pictographic(ch: char) -> bool {
    use crate::tables::emoji::{self, EmojiCat};
    emoji::emoji_category(ch).2 == EmojiCat::EC_Extended_Pictographic
}
