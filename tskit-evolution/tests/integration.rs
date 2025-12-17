use neutral_evolution::{evolve, Parameters};
use tskit_evolution::*;

#[test]
fn test_simulation_round_trip() {
    for simplification_interval in 1..6_i64 {
        let mut t = EvolvableTableCollection::new(100, 10, simplification_interval).unwrap();
        let p = Parameters::new(1.0, 1e-3, 100).unwrap();
        evolve(101, p, &mut t).unwrap();
        let ts = tskit::TreeSequence::try_from(t).unwrap();
        let _ = ts.dump_tables().unwrap();
    }
}

#[test]
fn test_simulation_round_trip_overlapping_gens() {
    for pdeath in [0.25, 0.5, 0.75, 0.9] {
        for simplification_interval in 1..6_i64 {
            let mut t = EvolvableTableCollection::new(100, 10, simplification_interval).unwrap();
            let p = Parameters::new(pdeath, 1e-1, 100).unwrap();
            evolve(101, p, &mut t).unwrap();
            let ts = tskit::TreeSequence::try_from(t).unwrap();
            let _ = ts.dump_tables().unwrap();
        }
    }
}
