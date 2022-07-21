pub use ancestry_common::{LargeSignedInteger, SignedInteger};

mod ancestry_overlapper;
mod error;
mod flags;
mod node_heap;
mod propagate_ancestry_changes;
mod segments;
mod update_ancestry;
mod util;

pub(crate) use ancestry_overlapper::AncestryOverlapper;
pub(crate) use segments::*;

pub mod node;
pub mod population;

// Public API
// NOTE: this API is TBD, and may later
// be exported via a pub mod.
pub use error::InlineAncestryError;
pub use flags::NodeFlags;
pub use node::Node;
pub use node::NodeData;
pub use node_heap::NodeHeap;
pub use population::Population;

pub mod indexed_node;
pub mod indexed_population;
