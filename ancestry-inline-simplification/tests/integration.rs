use ancestry_inline_simplification::*;
use neutral_evolution::{evolve, Parameters};
use tskit::prelude::*;
use tskit_evolution::EvolvableTableCollection;

#[test]
fn test_simulation_round_trip_nonoverlapping_gens() {
    for seed in [101, 201, 301, 401, 8512389, 12853581239, 95192] {
        // number of haploids, genome length
        let mut pop = Population::new(10, 100).unwrap();
        let mut evolveable_tables =
            tskit_evolution::EvolvableTableCollection::new(100, 10, 10).unwrap();
        // death rate, mean no. crossovers, no. steps to sim
        let p = Parameters::new(1.0, 1e-3, 250).unwrap();
        evolve(seed, p, &mut pop).unwrap();
        evolve(seed, p, &mut evolveable_tables).unwrap();
        assert!(pop.nodes.iter().any(|i| i.borrow().parents.len() > 0));
        for i in pop.nodes.iter() {
            assert_eq!(i.borrow().birth_time, 250);
            let mut stack = vec![i.clone()];
            while let Some(node) = stack.pop() {
                for p in &node.borrow().parents {
                    stack.push(p.clone());
                }
            }
        }
        let r = pop.all_reachable_nodes();
        let tables = tskit::TableCollection::from(evolveable_tables);
        assert_eq!(r.len(), usize::try_from(tables.nodes().num_rows()).unwrap());

        let pop_tables = tskit::TableCollection::try_from(pop).unwrap();
        assert_eq!(pop_tables.nodes().num_rows(), tables.nodes().num_rows());
        assert_eq!(pop_tables.edges().num_rows(), tables.edges().num_rows());
        {
            let tcopy = tables.deepcopy().unwrap();
            let pcopy = pop_tables.deepcopy().unwrap();
            let tseq = tcopy
                .tree_sequence(tskit::TreeSequenceFlags::BUILD_INDEXES)
                .unwrap();
            let pseq = pcopy
                .tree_sequence(tskit::TreeSequenceFlags::BUILD_INDEXES)
                .unwrap();
            assert_eq!(tseq.num_trees(), pseq.num_trees(), "{}", seed);
        }
    }
}

#[test]
fn test_simulation_round_trip_overlapping_gens() {
    for seed in [101, 201, 301, 401, 8512389, 12853581239, 95192] {
        for pdeath in [0.25, 0.5, 0.75, 0.9] {
            let mut pop = Population::new(10, 100).unwrap();
            let mut evolveable_tables =
                tskit_evolution::EvolvableTableCollection::new(100, 10, 10).unwrap();
            let p = Parameters::new(pdeath, 1e-1, 250).unwrap();
            evolve(seed, p, &mut pop).unwrap();
            evolve(seed, p, &mut evolveable_tables).unwrap();
            assert!(pop.nodes.iter().any(|i| i.borrow().parents.len() > 0));
            let r = pop.all_reachable_nodes();
            let tables = tskit::TableCollection::from(evolveable_tables);
            assert_eq!(r.len(), usize::try_from(tables.nodes().num_rows()).unwrap());
            let pop_tables = tskit::TableCollection::try_from(pop).unwrap();

            // does simplify help?
            {
                let mut tcopy = tables.deepcopy().unwrap();
                let mut tsamples = vec![];
                for node in tcopy.nodes_iter() {
                    if node.flags.is_sample() {
                        tsamples.push(node.id);
                    }
                }
                //tcopy
                //    .simplify(&tsamples, tskit::SimplificationOptions::default(), false)
                //    .unwrap();
                let mut pcopy = pop_tables.deepcopy().unwrap();
                let mut samples = vec![];
                for node in pcopy.nodes_iter() {
                    if node.flags.is_sample() {
                        samples.push(node.id);
                    }
                }
                pcopy
                    .simplify(&samples, tskit::SimplificationOptions::default(), false)
                    .unwrap();
                assert!(tsamples.len() > 0);
                assert_eq!(tsamples.len(), samples.len());
                assert_eq!(
                    pcopy.edges().num_rows(),
                    tcopy.edges().num_rows(),
                    "{} {}",
                    seed,
                    pdeath
                );
                assert_eq!(pcopy.nodes().num_rows(), pop_tables.nodes().num_rows());
                // FIXME: this should pass, but does not.
                // We are not fully simplified for overlapping generations?
                //assert_eq!(pcopy.edges().num_rows(), pop_tables.edges().num_rows());

                let ts = tcopy
                    .tree_sequence(tskit::TreeSequenceFlags::BUILD_INDEXES)
                    .unwrap();
                let pts = pcopy
                    .tree_sequence(tskit::TreeSequenceFlags::BUILD_INDEXES)
                    .unwrap();
                assert_eq!(ts.num_trees(), pts.num_trees());
            }

            //{
            //    let tcopy = tables.deepcopy().unwrap();
            //    let pcopy = pop_tables.deepcopy().unwrap();
            //    let tseq = tcopy
            //        .tree_sequence(tskit::TreeSequenceFlags::BUILD_INDEXES)
            //        .unwrap();
            //    let pseq = pcopy
            //        .tree_sequence(tskit::TreeSequenceFlags::BUILD_INDEXES)
            //        .unwrap();
            //    assert_eq!(tseq.num_trees(), pseq.num_trees(), "{} {}", seed, pdeath);
            //}

            //assert_eq!(pop_tables.nodes().num_rows(), tables.nodes().num_rows());
            //if tables.edges().num_rows() != pop_tables.edges().num_rows() {
            //    for i in tables.edges_iter() {
            //        println!(
            //            "{} {} {} {}",
            //            i.left,
            //            i.right,
            //            tables.nodes().time(i.parent).unwrap(),
            //            tables.nodes().time(i.child).unwrap(),
            //        );
            //    }
            //    for i in pop_tables.edges_iter() {
            //        println!(
            //            "pop: {} {} {} {}",
            //            i.left,
            //            i.right,
            //            pop_tables.nodes().time(i.parent).unwrap(),
            //            pop_tables.nodes().time(i.child).unwrap()
            //        );
            //    }
            //}
            //assert_eq!(
            //    pop_tables.edges().num_rows(),
            //    tables.edges().num_rows(),
            //    "{} {}",
            //    seed,
            //    pdeath
            //);
        }
    }
}
