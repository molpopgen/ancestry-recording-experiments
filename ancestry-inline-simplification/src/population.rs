use crate::node::Node;
use crate::node_heap::NodeHeap;
use crate::InlineAncestryError;
use crate::LargeSignedInteger;
use crate::SignedInteger;
use hashbrown::HashSet;
use neutral_evolution::EvolveAncestry;
use tskit::prelude::*;

pub struct Population {
    next_node_id: SignedInteger,
    genome_length: LargeSignedInteger,
    replacements: Vec<usize>,
    births: Vec<Node>,
    next_replacement: usize,
    node_heap: NodeHeap,
    pub nodes: Vec<Node>,
}

impl Population {
    pub fn new(
        popsize: SignedInteger,
        genome_length: LargeSignedInteger,
    ) -> Result<Self, InlineAncestryError> {
        if genome_length > 0 {
            let next_node_id = popsize;

            let mut nodes = vec![];

            for i in 0..next_node_id {
                let node = Node::new_alive_with_ancestry_mapping_to_self(i, 0, genome_length);
                nodes.push(node);
            }

            Ok(Self {
                next_node_id,
                genome_length,
                replacements: vec![],
                births: vec![],
                next_replacement: 0,
                node_heap: NodeHeap::default(),
                nodes,
            })
        } else {
            Err(InlineAncestryError::InvalidGenomeLength { l: genome_length })
        }
    }

    pub fn birth(&mut self, birth_time: LargeSignedInteger) -> Node {
        assert!(birth_time >= 0);
        let index = self.next_node_id;
        self.next_node_id += 1;
        Node::new_alive_with_ancestry_mapping_to_self(index, birth_time, self.genome_length)
    }

    pub fn get(&self, who: usize) -> Option<&Node> {
        self.nodes.get(who)
    }

    pub fn get_mut(&mut self, who: usize) -> Option<&mut Node> {
        self.nodes.get_mut(who)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn all_reachable_nodes(&self) -> HashSet<Node> {
        crate::util::all_reachable_nodes(&self.nodes)
    }

    pub fn num_still_reachable(&self) -> usize {
        self.all_reachable_nodes().len()
    }

    pub fn validate_graph(&self) -> Result<(), InlineAncestryError> {
        crate::util::validate_graph(&self.nodes, self.genome_length)
    }
}

impl EvolveAncestry for Population {
    fn genome_length(&self) -> LargeSignedInteger {
        self.genome_length
    }

    fn setup(&mut self, _final_time: LargeSignedInteger) {}

    fn generate_deaths(&mut self, death: &mut neutral_evolution::Death) -> usize {
        self.replacements.clear();
        self.next_replacement = 0;

        for i in 0..self.nodes.len() {
            if death.dies() {
                self.replacements.push(i);
            }
        }

        self.replacements.len()
    }

    fn current_population_size(&self) -> usize {
        self.nodes.len()
    }

    fn record_birth(
        &mut self,
        birth_time: LargeSignedInteger,
        _final_timepoint: LargeSignedInteger,
        breakpoints: &[neutral_evolution::TransmittedSegment],
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert!(!breakpoints.is_empty());
        // Give birth to a new Individual ("node")
        let mut birth = self.birth(birth_time);

        for b in breakpoints {
            // Increase ref count of parent
            let mut parent = self.get_mut(b.parent).as_mut().unwrap().clone();

            // Add references to birth for each segment
            parent.add_child_segment(b.left, b.right, birth.clone())?;
            // MOVE parent w/o increasing ref count
            birth.add_parent(parent)?;
        }

        assert!(!birth.borrow().parents.is_empty());

        // MOVE the birth w/o increasing ref count
        self.births.push(birth);
        Ok(())
    }

    fn simplify(
        &mut self,
        current_time_point: LargeSignedInteger,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(self.replacements.len(), self.births.len());
        assert!(self.node_heap.is_empty());

        for (i, death) in self.replacements.iter().enumerate() {
            let dead = self.nodes[*death].clone();
            let birth = self.births[i].clone();
            assert_eq!(self.births[i].borrow().birth_time, current_time_point);
            assert!(self.nodes[*death].is_alive());
            self.node_heap.push_death(dead)?;
            self.node_heap.push_birth(birth.clone())?;

            self.nodes[*death] = birth;
        }

        self.births.clear();

        let _poppped = crate::propagate_ancestry_changes::propagate_ancestry_changes(
            self.genome_length,
            &mut self.node_heap,
        )?;

        #[cfg(debug_assertions)]
        {
            self.validate_graph()?;
        }

        assert!(self.node_heap.is_empty());
        Ok(())
    }

    fn finish(
        &mut self,
        _current_time_point: LargeSignedInteger,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl TryFrom<Population> for tskit::TableCollection {
    type Error = crate::InlineAncestryError;

    fn try_from(value: Population) -> Result<Self, Self::Error> {
        let mut tables = match tskit::TableCollection::new(value.genome_length() as f64) {
            Ok(tables) => tables,
            Err(e) => return Err(crate::InlineAncestryError::TskitError(e)),
        };

        let mut node_map = std::collections::HashMap::<_, _>::default();
        let mut max_time: LargeSignedInteger = 0;

        for i in value.all_reachable_nodes() {
            max_time = std::cmp::max(max_time, i.borrow().birth_time);
            node_map.insert(i.clone(), tskit::NodeId::NULL);
        }

        for (k, v) in node_map.iter_mut() {
            let birth_time = (-1_i64 * (k.borrow().birth_time - max_time)) as f64;
            *v = match tables.add_node(0, birth_time, -1, -1) {
                Ok(node_id) => node_id,
                Err(e) => return Err(crate::InlineAncestryError::TskitError(e)),
            };
        }

        for i in value.all_reachable_nodes() {
            let pid = node_map.get(&i).unwrap();
            for (k, v) in i.borrow().children.iter() {
                let cid = node_map.get(&k).unwrap();
                for j in v {
                    match tables.add_edge(j.left as f64, j.right as f64, *pid, *cid) {
                        Ok(_) => (),
                        Err(e) => return Err(crate::InlineAncestryError::TskitError(e)),
                    }
                }
            }
        }

        for i in value.nodes.iter() {
            let node = node_map.get(i).unwrap();
            tables.nodes_mut().flags_array_mut()[node.as_usize()] = tskit::NodeFlags::IS_SAMPLE;
        }

        match tables.full_sort(tskit::TableSortOptions::default()) {
            Ok(_) => (),
            Err(e) => return Err(crate::InlineAncestryError::TskitError(e)),
        }

        match tables.build_index() {
            Ok(_) => (),
            Err(e) => return Err(crate::InlineAncestryError::TskitError(e)),
        }
        Ok(tables)
    }
}
