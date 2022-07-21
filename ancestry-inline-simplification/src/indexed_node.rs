use crate::LargeSignedInteger;
use crate::NodeFlags;
use crate::Segment;

pub struct AncestrySegment {
    pub segment: Segment,
    pub child: usize,
}

pub type ChildMap = nohash_hasher::IntMap<usize, Vec<Segment>>;
pub type ParentSet = nohash_hasher::IntSet<usize>;

// NOTE: this is probably borrow-checker hell waiting to happen!
// This stuff will need to be broken up into several Vec<_>.
#[derive(Default)]
pub struct Node {
    /// Index of this node in the container
    pub index: usize,
    pub birth_time: LargeSignedInteger,
    pub flags: NodeFlags,
    pub parents: ParentSet,
    pub ancestry: Vec<AncestrySegment>,
    pub children: ChildMap,
}

#[derive(Default)]
pub struct NodeTable {
    pub index: Vec<usize>, // Redundant?
    pub counts: Vec<u32>,
    pub birth_time: Vec<LargeSignedInteger>,
    pub flags: Vec<NodeFlags>,
    pub parents: Vec<ParentSet>,
    pub ancestry: Vec<Vec<AncestrySegment>>,
    pub children: Vec<ChildMap>,
    queue: Vec<usize>, // for recycling
}

impl NodeTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_birth(
        &mut self,
        birth_time: LargeSignedInteger,
        genome_length: LargeSignedInteger,
        parents: ParentSet,
    ) -> usize {
        match self.queue.pop() {
            Some(index) => {
                self.counts[index] = 1;
                self.birth_time[index] = birth_time;
                self.flags[index] = NodeFlags::new_alive();
                self.parents[index] = parents;
                self.ancestry[index].clear();
                self.ancestry[index].push(AncestrySegment {
                    segment: Segment::new(0, genome_length).unwrap(),
                    child: index,
                });
                self.children[index].clear();

                index
            }
            None => {
                self.index.push(self.index.len());
                self.counts.push(1);
                self.birth_time.push(birth_time);
                self.flags.push(NodeFlags::new_alive());
                self.parents.push(parents);
                self.ancestry.push(vec![AncestrySegment {
                    segment: Segment::new(0, genome_length).unwrap(),
                    child: self.index.len() - 1,
                }]);
                self.children.push(ChildMap::default());

                self.index.len() - 1
            }
        }
    }
}

impl Node {
    // FIXME: fallible b/c of Segment::new
    pub(crate) fn new_birth(
        index: usize,
        birth_time: LargeSignedInteger,
        genome_length: LargeSignedInteger,
        parents: ParentSet,
    ) -> Self {
        Self {
            index,
            birth_time,
            flags: NodeFlags::new_alive(),
            parents,
            ancestry: vec![AncestrySegment {
                segment: Segment::new(0, genome_length).unwrap(),
                child: index,
            }],
            children: ChildMap::default(),
        }
    }

    // FIXME: fallible b/c of Segment::new
    pub(crate) fn recycle(
        &mut self,
        birth_time: LargeSignedInteger,
        genome_length: LargeSignedInteger,
        parents: ParentSet,
    ) {
        self.ancestry.clear();
        self.children.clear();
        let mut p = parents;
        std::mem::swap(&mut self.parents, &mut p);
        self.ancestry.push(AncestrySegment {
            segment: Segment::new(0, genome_length).unwrap(),
            child: self.index,
        });
        self.birth_time = birth_time;
    }
}
