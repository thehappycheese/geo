/// I am trying to get a custom iterator working to replace the
/// [super::slice_itertools::pairwise()] function.
///
/// It is turning out to be very complicated :(
///
///  My requirements are
///
///  - Facilitate iterating over `Line`s in a LineString in a pairwise fashion
///  - Offset the `Line` inside the iterator
///  - Avoid repeatedly calculating length for each line
///  - Make iterator lazier (don't keep all offset `Line`s in memory)
///  - Iterator should provide
///    - the offset points
///    - the intersection point ([LineIntersectionResultWithRelationships])
///    - the pre-calculated length of offset line segments (for miter limit
///      calculation)
///    - support wrapping over to the first segment at the end to simplify
///      closed shapes
///
use crate::{Coord, CoordFloat, CoordNum, LineString};

use super::line_intersection::{
    line_segment_intersection_with_relationships, LineIntersectionResultWithRelationships,
};
use super::offset_line_raw::{offset_line_raw, OffsetLineRawResult};

/// Bring this into scope to imbue [LineString] with
/// [LineStringOffsetSegmentPairIterable::iter_offset_segment_pairs()]
pub(super) trait LineStringOffsetSegmentPairs<T>
where
    T: CoordFloat,
{
    /// Loop over the segments of a [LineString] in a pairwise fashion,
    /// offsetting and intersecting them as we go.
    /// 
    /// Returns an [OffsetSegmentsIterator]
    fn iter_offset_segment_pairs(&self, distance: T) -> OffsetSegmentsIterator<T>;
}

pub(super) struct OffsetSegmentsIterator<'a, T>
where
    T: CoordFloat,
{
    line_string: &'a LineString<T>,
    distance: T,
    previous_offset_segment: Option<OffsetLineRawResult<T>>,
    index: usize,
}

impl<T> LineStringOffsetSegmentPairs<T> for LineString<T>
where
    T: CoordFloat,
{
    fn iter_offset_segment_pairs(&self, distance: T) -> OffsetSegmentsIterator<T>
    where
        T: CoordNum,
    {
        if self.0.len() < 3 {
            // LineString is not long enough, therefore return an iterator that
            // will return None as first result
            OffsetSegmentsIterator {
                line_string: self,
                distance,
                previous_offset_segment: None,
                index: 0,
            }
        } else {
            // TODO: Length check above prevents panic; use
            // unsafe get_unchecked for performance?
            let a = self.0[0];
            let b = self.0[1];
            OffsetSegmentsIterator {
                line_string: self,
                distance,
                previous_offset_segment: offset_line_raw(a, b, distance),
                index: 0,
            }
        }
    }
}

///
/// The following diagram illustrates the meaning of the struct members.
/// The `LineString` `abc` is offset to form the `Line`s `mn` and `op`.
/// `i` is the intersection point.
///
/// ```text
///          a
///  m        \
///   \        \
///    \        b---------c
///     n
///
///        i    o---------p
/// ```
#[derive(Clone)]
pub(super) struct OffsetSegmentsIteratorItem<T>
where
    T: CoordNum,
{
    pub a: Coord<T>,
    pub b: Coord<T>,
    pub c: Coord<T>,

    pub m: Coord<T>,
    pub n: Coord<T>,
    pub o: Coord<T>,
    pub p: Coord<T>,

    /// Distance between `a` and `b` (same as distance between `m` and `n`)
    pub ab_len: T,
    /// Distance between `b` and `c` (same as distance between `o` and `p`)
    pub bc_len: T,

    /// Intersection [Coord] between segments `mn` and `op`
    pub i: Option<LineIntersectionResultWithRelationships<T>>,
}

impl<'a, T> Iterator for OffsetSegmentsIterator<'a, T>
where
    T: CoordFloat,
{
    /// Option since each step of the iteration may fail.
    type Item = Option<OffsetSegmentsIteratorItem<T>>;

    /// Return type is confusing; `Option<Option<OffsetSegmentsIteratorItem<T>>>`
    /// 
    /// The outer Option is required by the Iterator trait, and indicates if
    /// iteration is finished, (When this iterator is used via `.map()` or
    /// similar the user does not see the outer Option.)
    /// The inner Option indicates if the result of each iteration is valid.
    /// Returning None will halt iteration, returning Some(None) will not,
    /// but the user should stop iterating.
    /// 
    fn next(&mut self) -> Option<Self::Item> {
        if self.index + 3 > self.line_string.0.len() {
            // Iteration is complete
            return None;
        } else {
            // TODO: Length check above prevents panic; use
            // unsafe get_unchecked for performance?
            let a = self.line_string[self.index];
            let b = self.line_string[self.index + 1];
            let c = self.line_string[self.index + 2];

            self.index += 1;

            // Fetch previous offset segment
            let Some(OffsetLineRawResult{
                a_offset:m,
                b_offset:n,
                ab_len,
            }) = self.previous_offset_segment else {
                return None
            };

            // Compute next offset segment
            self.previous_offset_segment = offset_line_raw(b, c, self.distance);
            let Some(OffsetLineRawResult{
                a_offset:o,
                b_offset:p,
                ab_len:bc_len,
            }) = self.previous_offset_segment else {
                return Some(None);
            };

            Some(Some(
                OffsetSegmentsIteratorItem {
                    a,
                    b,
                    c,
                    m, // TODO < replace mnop and ab_len and bc_len with two optional OffsetLineRawResult and remove the Option form Self::Item
                    n,#
                    o,
                    p,
                    ab_len,
                    bc_len,
                    i:line_segment_intersection_with_relationships(&m, &n, &o, &p),
                }
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use super::{LineStringOffsetSegmentPairs, OffsetSegmentsIteratorItem};
    use crate::{
        line_string, offset_curve::line_intersection::LineIntersectionResultWithRelationships,
        Coord,
    };

    #[test]
    fn test_iterator() {
        let input = line_string![
            Coord { x: 1f64, y: 0f64 },
            Coord { x: 1f64, y: 1f64 },
            Coord { x: 2f64, y: 1f64 },
        ];

        let result: Option<Vec<()>> = input
            .iter_offset_segment_pairs(1f64)
            .map(|item| match item {
                Some(OffsetSegmentsIteratorItem {
                    a,
                    b,
                    c,

                    m,
                    n,
                    o,
                    p,

                    ab_len,
                    bc_len,

                    i:
                        Some(LineIntersectionResultWithRelationships {
                            ab,
                            cd,
                            intersection,
                        }),
                }) => Some(()),
                _ => None,
            })
            .collect();
        assert!(result.unwrap().len()==1);
    }
}
