use crate::{individual::Individual, LargeSignedInteger, SignedInteger};
use std::cmp::Ordering;
use std::rc::Rc;
use std::{cell::RefCell, ops::Deref};

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct Overlap {
    pub left: LargeSignedInteger,
    pub right: LargeSignedInteger,
    pub child: Individual,
    pub mapped_individual: Individual,
}

impl Overlap {
    pub fn new(
        left: LargeSignedInteger,
        right: LargeSignedInteger,
        child: Individual,
        mapped_individual: Individual,
    ) -> Self {
        assert!(left < right, "{} {}", left, right);
        Self {
            left,
            right,
            child,
            mapped_individual,
        }
    }
}

pub(crate) struct AncestryOverlapper {
    segments: Vec<Overlap>,
    overlaps: Rc<RefCell<Vec<Overlap>>>, // Prevents copying the segments over and over
    j: usize,
    n: usize,
    right: LargeSignedInteger,
}

impl AncestryOverlapper {
    // FIXME: should Err if input are not sorted.
    pub(crate) fn new(segments: Vec<Overlap>) -> Self {
        let mut segments = segments;
        let n = segments.len();
        let overlaps = vec![];

        segments.sort();
        // Sentinel
        segments.push(Overlap::new(
            LargeSignedInteger::MAX - 1,
            LargeSignedInteger::MAX,
            // NOTE: dummy individual here to avoid using Option globally for
            // child field of Overlap
            Individual::new(SignedInteger::MAX, LargeSignedInteger::MAX),
            Individual::new(SignedInteger::MAX, LargeSignedInteger::MAX),
        ));
        let sorted = segments.windows(2).all(|w| w[0].left <= w[1].left);
        assert!(sorted);
        let right = segments[0].left;
        Self {
            segments,
            overlaps: Rc::new(RefCell::new(overlaps)),
            j: 0,
            n,
            right,
        }
    }
}

impl Iterator for AncestryOverlapper {
    type Item = (
        LargeSignedInteger,
        LargeSignedInteger,
        Rc<RefCell<Vec<Overlap>>>,
    );

    fn next(&mut self) -> Option<Self::Item> {
        if self.j < self.n {
            let mut left = self.right;
            self.overlaps.borrow_mut().retain(|x| x.right > left);
            if self.overlaps.borrow().is_empty() {
                left = self.segments[self.j].left;
            }
            while self.j < self.n && self.segments[self.j].left == left {
                self.overlaps
                    .borrow_mut()
                    .push(self.segments[self.j].clone());
                self.j += 1;
            }
            self.j -= 1;
            self.right = self
                .overlaps
                .borrow()
                .iter()
                .fold(LargeSignedInteger::MAX, |a, b| std::cmp::min(a, b.right));
            self.right = std::cmp::min(self.right, self.segments[self.j + 1].right);
            self.j += 1;
            return Some((left, self.right, self.overlaps.clone()));
        }

        if !self.overlaps.borrow().is_empty() {
            let left = self.right;
            self.overlaps.borrow_mut().retain(|x| x.right > left);
            if !self.overlaps.borrow().is_empty() {
                self.right = self
                    .overlaps
                    .borrow()
                    .iter()
                    .fold(LargeSignedInteger::MAX, |a, b| std::cmp::min(a, b.right));
                return Some((left, self.right, self.overlaps.clone()));
            }
        }

        None

        // TODO: see of this code also works.  It is a cleaner way to do, I think.
        //if !self.segments.is_empty() {
        //    let mut left = self.right;
        //    self.overlaps.borrow_mut().retain(|x| x.right > left);
        //    if self.overlaps.borrow().is_empty() {
        //        left = self.segments.last().unwrap().left;
        //    }
        //    while !self.segments.is_empty() && self.segments.last().unwrap().left == left {
        //        let x = self.segments.pop().unwrap();
        //        self.overlaps.borrow_mut().push(x);
        //    }
        //    self.right = self
        //        .overlaps
        //        .borrow()
        //        .iter()
        //        .fold(LargeSignedInteger::MAX, |a, b| std::cmp::min(a, b.right));
        //    if let Some(seg) = self.segments.last() {
        //        self.right = std::cmp::min(self.right, seg.right);
        //    }
        //}
    }
}

impl Ord for Overlap {
    fn cmp(&self, other: &Self) -> Ordering {
        self.left.cmp(&other.left)
    }
}

impl PartialOrd for Overlap {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sorting() {
        let mut v = vec![
            Overlap::new(3, 4, Individual::new(1, 1), Individual::new(1, 2)),
            Overlap::new(2, 3, Individual::new(1, 2), Individual::new(1, 2)),
            Overlap::new(1, 2, Individual::new(1, 3), Individual::new(1, 2)),
        ];
        v.sort();
        assert!(v.windows(2).all(|w| w[0].left < w[1].left));
    }
}

#[cfg(test)]
mod overlapper_tests {
    use super::*;
    use crate::segment::Segment;

    #[test]
    fn test_single_overlap() {
        let mut parent = Individual::new(0, 0);

        let child1 = Individual::new(1, 1);
        let child2 = Individual::new(2, 1);

        {
            child1
                .borrow_mut()
                .ancestry
                .push(Segment::new(0, 5, child1.clone()));
            child2
                .borrow_mut()
                .ancestry
                .push(Segment::new(1, 6, child2.clone()));
        }

        parent.add_child_segment(0, 5, child1.clone());
        parent.add_child_segment(1, 6, child2.clone());

        let overlapper = AncestryOverlapper::new(parent.intersecting_ancestry());

        let expected = vec![vec![0, 5], vec![1, 6]];

        for (i, (left, right, _overlaps)) in overlapper.enumerate() {
            assert_eq!(expected[i][0], left);
            assert_eq!(expected[i][1], right);
        }
    }
}