mod point;
pub use point::SweepPoint;

mod events;
pub(crate) use events::{Event, EventType};

mod line_or_point;
pub use line_or_point::LineOrPoint;

mod cross;
pub use cross::Cross;

mod segment;
use segment::{Segment, SplitSegments};

mod active;
use active::{Active, ActiveSet};

mod im_segment;
use im_segment::IMSegment;

mod proc;
use proc::Sweep;

mod iter;
pub use iter::Intersections;
pub(crate) use iter::{Crossing, CrossingsIter};
