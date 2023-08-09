use geo_types::{CoordFloat, Line, LineString};

use super::{measure_line_string, LineSplit, LineSplitResult, LineStringMeasurements};

impl<Scalar> LineSplit<Scalar> for LineString<Scalar>
where
    Scalar: CoordFloat + std::iter::Sum,
{
    fn line_split(&self, fraction: Scalar) -> Option<LineSplitResult<Self>> {
        // import enum variants
        use LineSplitResult::*;
        if fraction.is_nan() {
            return None;
        }
        match (fraction <= Scalar::zero(), fraction >= Scalar::one()) {
            (false, true) => Some(First(self.clone())),
            (true, false) => Some(Second(self.clone())),
            _ => {
                // find the total length, and at the same time the length of each segment
                // TODO: consider the possibility of a `LineStringMeasured` datatype in the future
                //       as this will be a common requirement in several algorithms, and would be a big
                //       performance boost when repeatedly slicing portions from the same LineStrings
                //       I think I saw a PreparedGeometry PR? maybe that will cover this?
                let LineStringMeasurements {
                    length_total,
                    length_segments,
                } = match measure_line_string(&self) {
                    Some(x) => x,
                    None => return None,
                };

                if !Scalar::is_finite(length_total) || Scalar::is_zero(&length_total) {
                    // TODO: Does this cover a linestring with zero or one points?
                    return None;
                }

                // Find the length of the first part of the line string before the split;
                let length_fraction = fraction * length_total;
                // Set up some variables to track state in the for-loop
                let mut length_accumulated = Scalar::zero();
                // TODO: unwrap used; but should be safe since we check the length above
                let mut coords_first_part = vec![*self.0.first().unwrap()];
                let mut coords_second_part = Vec::new();

                // Convert window slices to tuples because destructuring slices of unknown length is not
                // possible
                // TODO: the itertools crate has a pairwise function which returns tuples
                let pairs = self.0.as_slice().windows(2).map(|item| (item[0], item[1]));

                for ((a, b), &length_segment) in pairs.zip(length_segments.iter()) {
                    let length_accumulated_before_segment = length_accumulated;
                    length_accumulated = length_accumulated + length_segment;
                    let length_accumulated_after_segment = length_accumulated;
                    if length_fraction < length_accumulated_before_segment {
                        coords_second_part.push(b);
                    } else if length_fraction >= length_accumulated_after_segment {
                        coords_first_part.push(b);
                    } else {
                        // TODO: check for divide by zero
                        let fraction_to_split_segment =
                            (length_fraction - length_accumulated_before_segment) / length_segment;
                        match Line::new(a, b).line_split(fraction_to_split_segment) {
                            Some(FirstSecond(line1, _line2)) => {
                                coords_first_part.push(line1.end);
                                coords_second_part.push(line1.end);
                                coords_second_part.push(b);
                            }
                            Some(First(_line1)) => {
                                coords_first_part.push(b);
                                coords_second_part.push(b);
                            }
                            Some(Second(_line2)) => {
                                coords_second_part.push(a);
                                coords_second_part.push(b);
                            }
                            None => return None, // probably never?
                        }
                    }
                }
                Some(FirstSecond(
                    coords_first_part.into(),
                    coords_second_part.into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use geo_types::{line_string, coord};

    use super::super::LineSplitTwiceResult;

    use super::*;
    // =============================================================================================
    // LineString::line_split()
    // =============================================================================================

    #[test]
    fn split() {
        let line_string:LineString<f32> = line_string![
            (x:0.0, y:0.0),
            (x:1.0, y:0.0),
            (x:1.0, y:1.0),
            (x:2.0, y:1.0),
        ];
        let mid_line = line_string.lines().nth(1).unwrap();
        let slice_point = mid_line.start + mid_line.delta() * 0.5;
        assert_eq!(
            line_string.line_split(0.5),
            Some(LineSplitResult::FirstSecond(
                LineString::new(vec![line_string.0[0],line_string.0[1], slice_point]),
                LineString::new(vec![slice_point, line_string.0[2],line_string.0[3]])
            ))
        );
    }
    
    #[test]
    fn split_on_point() {
        let line_string:LineString<f32> = line_string![
            (x:0.0, y:0.0),
            (x:1.0, y:0.0),
            (x:1.0, y:1.0),
            (x:2.0, y:1.0),
            (x:2.0, y:2.0),
        ];
        let slice_point = coord! {x:1.0, y:1.0};
        assert_eq!(
            line_string.line_split(0.5),
            Some(LineSplitResult::FirstSecond(
                LineString::new(vec![line_string.0[0],line_string.0[1], slice_point]),
                LineString::new(vec![slice_point, line_string.0[3], line_string.0[4]])
            ))
        );
    }

    #[test]
    fn split_half_way_through_last_segment() {
        let line_string:LineString<f32> = line_string![
            (x:0.0, y:0.0),
            (x:1.0, y:0.0),
            (x:1.0, y:1.0),
        ];
        let slice_point = coord! {x:1.0, y:0.5};
        assert_eq!(
            line_string.line_split(0.75),
            Some(LineSplitResult::FirstSecond(
                LineString::new(vec![line_string.0[0], line_string.0[1], slice_point]),
                LineString::new(vec![slice_point, line_string.0[2]])
            ))
        );
    }

    #[test]
    fn split_half_way_through_first_segment() {
        let line_string:LineString<f32> = line_string![
            (x:0.0, y:0.0),
            (x:1.0, y:0.0),
            (x:1.0, y:1.0),
        ];
        let slice_point = coord! {x:0.5, y:0.0};
        assert_eq!(
            line_string.line_split(0.25),
            Some(LineSplitResult::FirstSecond(
                LineString::new(vec![line_string.0[0], slice_point]),
                LineString::new(vec![slice_point, line_string.0[1], line_string.0[2]])
            ))
        );
    }

    #[test]
    fn split_first() {
        let line_string:LineString<f32> = line_string![
            (x:0.0, y:0.0),
            (x:1.0, y:0.0),
        ];
        assert_eq!(
            line_string.line_split(1.0),
            Some(LineSplitResult::First(line_string))
        );
    }

    #[test]
    fn split_second() {
        let line_string:LineString<f32> = line_string![
            (x:0.0, y:0.0),
            (x:1.0, y:0.0),
        ];
        assert_eq!(
            line_string.line_split(0.0),
            Some(LineSplitResult::Second(line_string))
        );
    }

    

    // =============================================================================================
    // LineString::line_split_twice()
    // =============================================================================================
    #[test]
    fn split_twice_typical(){
        // I think  if we exhaustively check
        // - `Line::line_split_twice()` and
        // - `LineString::line_split()`
        // then because the implementation for `line_split_twice` is shared
        // we don't need an exhaustive check for `LineString::line_split_twice()`
        // So I will just do a spot check for a typical case

        let line_string:LineString<f32> = line_string![
            (x:0.0, y:0.0),
            (x:1.0, y:0.0),
            (x:1.0, y:1.0),
            (x:2.0, y:1.0),
            (x:2.0, y:2.0),
        ];
        let result = line_string.line_split_twice(0.25, 0.5).unwrap();
        assert_eq!(
            result,
            LineSplitTwiceResult::FirstSecondThird(
                line_string![
                    (x: 0.0, y:0.0_f32),
                    (x: 1.0, y:0.0_f32),
                ],
                line_string![
                    (x: 1.0, y:0.0_f32),
                    (x: 1.0, y:1.0_f32),
                ],
                line_string![
                    (x: 1.0, y:1.0_f32),
                    (x: 2.0, y:1.0_f32),
                    (x: 2.0, y:2.0_f32),
                ],
            )
        );
    }
}