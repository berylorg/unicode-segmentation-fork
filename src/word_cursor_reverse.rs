#[derive(Clone, Copy, Debug)]
struct ReverseState {
    significant: Option<Significant>,
    significant_offset: usize,
    right_pictographic: bool,
    skipped: bool,
    include_left: bool,
    pending: Option<ReversePending>,
    regional: Option<ReverseRegional>,
    leading_ignored_start: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
struct ReversePending {
    connector: WordCat,
    right: WordCat,
    right_offset: usize,
}

#[derive(Clone, Copy, Debug)]
struct ReverseRegional {
    odd: bool,
    rightmost_offset: usize,
    second_right_offset: usize,
}

impl ReverseState {
    fn new(cursor_start: usize) -> Self {
        Self {
            significant: None,
            significant_offset: cursor_start,
            right_pictographic: false,
            skipped: false,
            include_left: false,
            pending: None,
            regional: None,
            leading_ignored_start: None,
        }
    }

    fn push(&mut self, ch: char, at: usize) -> Option<usize> {
        let category = wd::word_category(ch).2;
        let pictographic = is_extended_pictographic(ch);

        if let Some(mut regional) = self.regional {
            if is_ignored(category) {
                self.skipped = true;
                return None;
            }
            if category == wd::WC_Regional_Indicator {
                regional.odd = !regional.odd;
                self.regional = Some(regional);
                return None;
            }
            return Some(regional_boundary(regional));
        }

        if self.significant.is_none() {
            if is_ignored(category) {
                self.skipped = true;
                self.leading_ignored_start = Some(at);
                return None;
            }
            if is_newline(category) {
                if let Some(boundary) = self.leading_ignored_start {
                    return Some(boundary);
                }
            }
            self.significant = Some(Significant::Cat(category));
            self.significant_offset = at;
            self.right_pictographic = pictographic;
            self.skipped = false;
            return None;
        }

        if is_ignored(category) {
            if category == wd::WC_ZWJ
                && matches!(self.significant, Some(Significant::Cat(_)))
                && self.right_pictographic
                && !self.skipped
            {
                self.include_left = true;
                self.significant = Some(Significant::Emoji);
            }
            self.skipped = true;
            return None;
        }

        if self.include_left {
            self.include_left = false;
            self.update_significant(category, at, pictographic);
            return None;
        }

        if let Some(pending) = self.pending {
            if connector_matches(category, pending.connector, pending.right) {
                self.pending = None;
                self.update_significant(category, at, pictographic);
                return None;
            }
            return Some(pending.right_offset);
        }

        let right = match self.significant.unwrap() {
            Significant::Emoji => return Some(self.significant_offset),
            Significant::Cat(category) => category,
        };

        if is_hard_break(category, right) {
            return Some(self.significant_offset);
        }
        if category == wd::WC_CR && right == wd::WC_LF && !self.skipped {
            self.update_significant(category, at, pictographic);
            return None;
        }
        if category == wd::WC_WSegSpace && right == wd::WC_WSegSpace {
            if !self.skipped {
                self.update_significant(category, at, pictographic);
                return None;
            }
            return Some(self.significant_offset);
        }
        if is_connector_before(category, right) {
            self.pending = Some(ReversePending {
                connector: category,
                right,
                right_offset: self.significant_offset,
            });
            self.skipped = false;
            return None;
        }
        if category == wd::WC_Regional_Indicator && right == wd::WC_Regional_Indicator {
            self.regional = Some(ReverseRegional {
                odd: false,
                rightmost_offset: self.significant_offset,
                second_right_offset: at,
            });
            return None;
        }
        if pair_matches(category, right) {
            self.update_significant(category, at, pictographic);
            return None;
        }
        Some(self.significant_offset)
    }

    fn update_significant(&mut self, category: WordCat, at: usize, pictographic: bool) {
        self.significant = Some(Significant::Cat(category));
        self.significant_offset = at;
        self.right_pictographic = pictographic;
        self.skipped = false;
    }

    fn finish(&self) -> usize {
        if let Some(regional) = self.regional {
            return regional_boundary(regional);
        }
        if let Some(pending) = self.pending {
            return pending.right_offset;
        }
        if self.skipped && !self.include_left && self.significant.is_some() {
            return self.significant_offset;
        }
        0
    }
}

fn is_connector_before(connector: WordCat, right: WordCat) -> bool {
    (is_ahletter(right)
        && matches!(
            connector,
            wd::WC_MidLetter | wd::WC_MidNumLet | wd::WC_Single_Quote
        ))
        || (right == wd::WC_Hebrew_Letter && connector == wd::WC_Double_Quote)
        || (right == wd::WC_Numeric
            && matches!(
                connector,
                wd::WC_MidNum | wd::WC_MidNumLet | wd::WC_Single_Quote
            ))
}

fn regional_boundary(regional: ReverseRegional) -> usize {
    if !regional.odd {
        regional.second_right_offset
    } else {
        regional.rightmost_offset
    }
}
