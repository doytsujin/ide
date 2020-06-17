
use crate::prelude::*;

use crate::buffer::location::*;


// =================
// === Selection ===
// =================

/// Text selection. In case the `start` and `end` offsets are equal, the selection is interpreted as
/// a caret. The `column` field is a saved horizontal position used primarily for line up/down
/// movement. Please note that the start of the selection is not always smaller then its end.
/// If the selection was dragged from right to left, the start byte offset will be bigger than the
/// end. Use the `left` and `right` methods to discover the edges.
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
#[allow(missing_docs)]
pub struct Selection {
    pub start  : ByteOffset,
    pub end    : ByteOffset,
    pub column : Option<Column>,
}

impl Selection {
    /// Constructor.
    pub fn new(start:ByteOffset, end:ByteOffset) -> Self {
        let column = default();
        Self {start,end,column}
    }

    /// Gets the earliest offset within the selection, ie the minimum of both edges.
    pub fn min(self) -> ByteOffset {
        std::cmp::min(self.start, self.end)
    }

    /// Gets the latest offset within the selection, ie the maximum of both edges.
    pub fn max(self) -> ByteOffset {
        std::cmp::max(self.start, self.end)
    }

    /// Determines whether the selection is a caret (ie has an empty interior).
    pub fn is_caret(self) -> bool {
        self.start == self.end
    }

    /// Returns a selection with the given horizontal position.
    pub fn with_column(self, column:Option<Column>) -> Self {
        Self {column,..self}
    }

    // Indicate whether this region should merge with the next.
    // Assumption: regions are sorted (self.min() <= other.min())
    pub fn should_merge(self, other: Selection) -> bool {
        other.min() < self.max()
            || ((self.is_caret() || other.is_caret()) && other.min() == self.max())
    }

    // Merge self with an overlapping region.
    // Retains direction of self.
    pub fn merge_with(self, other: Selection) -> Selection {
        let is_forward = self.end >= self.start;
        let new_min = std::cmp::min(self.min(), other.min());
        let new_max = std::cmp::max(self.max(), other.max());
        let (start, end) = if is_forward { (new_min, new_max) } else { (new_max, new_min) };
        // Could try to preserve horiz/affinity from one of the
        // sources, but very likely not worth it.
        Selection::new(start, end)
    }

}



// =============
// === Group ===
// =============

/// A set of zero or more selection regions, representing a selection state.
#[derive(Clone,Debug,Default)]
pub struct Group {
    // An invariant: regions[i].max() <= regions[i+1].min()
    // and < if either is_caret()
    regions: Vec<Selection>,
}

/// Implementing the Deref trait allows callers to easily test `is_empty`, iterate
/// through all ranges, etc.
impl Deref for Group {
    type Target = [Selection];
    fn deref(&self) -> &[Selection] {
        &self.regions
    }
}

impl Group {
    /// Creates a new empty selection.
    pub fn new() -> Group {
        Group::default()
    }

    /// Add a region to the selection. This method implements merging logic.
    ///
    /// Two non-caret regions merge if their interiors intersect; merely
    /// touching at the edges does not cause a merge. A caret merges with
    /// a non-caret if it is in the interior or on either edge. Two carets
    /// merge if they are the same offset.
    ///
    /// Performance note: should be O(1) if the new region strictly comes
    /// after all the others in the selection, otherwise O(n).
    pub fn add_region(&mut self, region: Selection) {
        let mut ix = self.search(region.min());
        if ix == self.regions.len() {
            self.regions.push(region);
            return;
        }
        let mut region = region;
        let mut end_ix = ix;
        if self.regions[ix].min() <= region.min() {
            if self.regions[ix].should_merge(region) {
                region = region.merge_with(self.regions[ix]);
            } else {
                ix += 1;
            }
            end_ix += 1;
        }
        while end_ix < self.regions.len() && region.should_merge(self.regions[end_ix]) {
            region = region.merge_with(self.regions[end_ix]);
            end_ix += 1;
        }
        if ix == end_ix {
            self.regions.insert(ix, region);
        } else {
            self.regions[ix] = region;
            remove_n_at(&mut self.regions, ix + 1, end_ix - ix - 1);
        }
    }


    // The smallest index so that offset > region.max() for all preceding
    // regions.
    pub fn search(&self, offset:ByteOffset) -> usize {
        if self.regions.is_empty() || offset > self.regions.last().unwrap().max() {
            return self.regions.len();
        }
        match self.regions.binary_search_by(|r| r.max().cmp(&offset)) {
            Ok(ix) => ix,
            Err(ix) => ix,
        }
    }
}


pub fn remove_n_at<T: Clone>(v: &mut Vec<T>, index: usize, n: usize) {
    match n.cmp(&1) {
        std::cmp::Ordering::Equal => {
            v.remove(index);
        }
        std::cmp::Ordering::Greater => {
            let new_len = v.len() - n;
            for i in index..new_len {
                v[i] = v[i + n].clone();
            }
            v.truncate(new_len);
        }
        std::cmp::Ordering::Less => (),
    }
}