use crate::indexed_node::{Node, ParentSet};
use crate::InlineAncestryError;
use crate::LargeSignedInteger;
use crate::SignedInteger;

#[derive(Default)]
pub struct IndexedPopulation {
    nodes: Vec<crate::indexed_node::Node>,
    counts: Vec<i32>,
    // FIFO queue to recycle indexes of extinct (zero) counts
    queue: Vec<usize>,
    genome_length: LargeSignedInteger,
}

impl IndexedPopulation {
    pub fn new(
        popsize: SignedInteger,
        genome_length: LargeSignedInteger,
    ) -> Result<Self, InlineAncestryError> {
        if genome_length > 0 {
            let mut nodes = vec![];
            let mut counts = vec![];

            for i in 0..popsize {
                let node = Node::new_birth(i as usize, 0, genome_length, ParentSet::default());
                nodes.push(node);
                counts.push(1);
            }

            Ok(Self {
                nodes,
                counts,
                queue: vec![],
                genome_length,
            })
        } else {
            Err(InlineAncestryError::InvalidGenomeLength { l: genome_length })
        }
    }

    fn add_node(&mut self, birth_time: LargeSignedInteger, parent_indexes: &[usize]) -> usize {
        let mut parents = crate::indexed_node::ParentSet::default();
        for parent in parent_indexes {
            //FIXME: parents must exist...
            parents.insert(*parent);
            self.counts[*parent] += 1;
        }
        let rv = match self.queue.pop() {
            Some(index) => {
                // FIXME: this should pass on a set!
                self.nodes[index].recycle(birth_time, self.genome_length, parents);
                self.counts[index] += 1;
                index
            }
            None => {
                let index = self.nodes.len();
                self.nodes.push(crate::indexed_node::Node::new_birth(
                    index,
                    birth_time,
                    self.genome_length,
                    parents,
                ));
                self.counts.push(1);
                index
            }
        };
        debug_assert_eq!(self.nodes.len(), self.counts.len());
        rv
    }

    fn get_next_node_index(&mut self) -> usize {
        match self.queue.pop() {
            Some(value) => value,
            None => self.nodes.len(),
        }
    }
}

#[cfg(test)]
mod test_indexed_population {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut pop = IndexedPopulation::new(2, 10).unwrap();
        let birth_time: crate::LargeSignedInteger = 1;
        let parent_0 = 0_usize;
        let parent_1 = 1_usize;
        let b = pop.add_node(birth_time, &[parent_0, parent_1]);
        assert_eq!(b, 2);
        assert_eq!(pop.counts[parent_0], 2);
        assert_eq!(pop.counts[parent_1], 2);
    }

    #[test]
    fn test_forced_recycling() {
        let mut pop = IndexedPopulation::new(2, 10).unwrap();
        let birth_time: crate::LargeSignedInteger = 1;
        let parent_0 = 0_usize;
        let parent_1 = 1_usize;
        pop.queue.push(0); // FIXME: not working via public interface
        pop.add_node(birth_time, &[parent_0, parent_1]);
    }

    #[test]
    fn test_bad_parents() {
        let mut pop = IndexedPopulation::new(2, 10).unwrap();
        let birth_time: crate::LargeSignedInteger = 1;
        let parent_0 = 0_usize;
        assert!(pop.add_node(birth_time, &[parent_0]).is_err());
    }
}
