pub use ancestry_common::LargeSignedInteger;
use rand::prelude::Distribution;
use rand::SeedableRng;
use std::cell::RefCell;
use std::error::Error;
use std::ops::DerefMut;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParameterError {
    #[error("{0:?}")]
    BadParameter(String),
}

pub trait EvolveAncestry {
    fn genome_length(&self) -> LargeSignedInteger;

    fn setup(&mut self, final_time: LargeSignedInteger);

    /// Generate how many deaths (replacements) will occur at this time step.
    fn generate_deaths(&mut self, death: &mut Death) -> usize;

    /// For this prototype, we assume that pop size is
    /// constant during a sim
    fn current_population_size(&self) -> usize;

    /// Record a birth with the given parents and segments/edges.
    /// The details of generating a child are up to the specific type.
    fn record_birth(
        &mut self,
        birth_time: LargeSignedInteger,
        final_time: LargeSignedInteger,
        breakpoints: &[TransmittedSegment],
    ) -> Result<(), Box<dyn Error>>;

    fn simplify(&mut self, current_time_point: LargeSignedInteger) -> Result<(), Box<dyn Error>>;

    // When simplifying periodically, it is possible to not be simplified when the last time step
    // has finished. This fn lets the object clean up its ancestry.
    fn finish(&mut self, current_time_point: LargeSignedInteger) -> Result<(), Box<dyn Error>>;
}

type Rng = Rc<RefCell<rand_pcg::Pcg64>>;

pub struct Death {
    rng: Rng,
    death_probability: f64,
    uniform: rand::distributions::Uniform<f64>,
}

impl Death {
    fn new(death_probability: f64, rng: Rng) -> Self {
        Self {
            rng,
            death_probability,
            uniform: rand::distributions::Uniform::new(0., 1.),
        }
    }

    pub fn dies(&mut self) -> bool {
        self.uniform.sample(&mut self.rng.borrow_mut().deref_mut()) <= self.death_probability
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct TransmittedSegment {
    pub left: LargeSignedInteger,
    pub right: LargeSignedInteger,
    pub parent: usize,
}

impl TransmittedSegment {
    pub fn new(left: LargeSignedInteger, right: LargeSignedInteger, parent: usize) -> Self {
        Self {
            left,
            right,
            parent,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Parameters {
    death_probability: f64,
    mean_num_crossovers: f64,
    nsteps: LargeSignedInteger,
}

impl Parameters {
    pub fn new(
        death_probability: f64,
        mean_num_crossovers: f64,
        nsteps: LargeSignedInteger,
    ) -> Result<Self, ParameterError> {
        if !death_probability.is_finite() {
            return Err(ParameterError::BadParameter(
                "death_probability must be finite".to_string(),
            ));
        }
        if death_probability <= 0.0 || death_probability > 1.0 {
            return Err(ParameterError::BadParameter(
                "death_probability must be 0 < d <= 1.0".to_string(),
            ));
        }
        if !mean_num_crossovers.is_finite() {
            return Err(ParameterError::BadParameter(
                "mean_num_crossovers must be finite".to_string(),
            ));
        }
        if mean_num_crossovers < 0.0 {
            return Err(ParameterError::BadParameter(
                "mean_num_crossovers must be >= 0".to_string(),
            ));
        }
        if nsteps < 1 {
            return Err(ParameterError::BadParameter(
                "nsteps must be >= 1".to_string(),
            ));
        }
        Ok(Self {
            death_probability,
            mean_num_crossovers,
            nsteps,
        })
    }
}

fn fill_transmissions(
    parent1: usize,
    parent2: usize,
    crossovers: &[LargeSignedInteger],
    transmissions: &mut Vec<TransmittedSegment>,
) {
    transmissions.clear();
    let mut p1 = parent1;
    let mut p2 = parent2;
    let mut last_left = crossovers[0];
    let mut start = 1_usize;

    while start < crossovers.len() {
        let right = crossovers[start];
        let num = crossovers
            .iter()
            .skip(start)
            .take_while(|c| **c == right)
            .count();
        if num % 2 != 0 {
            transmissions.push(TransmittedSegment::new(last_left, right, p1));
            last_left = right;
            std::mem::swap(&mut p1, &mut p2);
        }
        start += num;
    }
}

fn generate_crossover_positions(
    genome_length: LargeSignedInteger,
    num_crossovers: u64,
    crossover_position: &rand_distr::Uniform<LargeSignedInteger>,
    rng: &mut rand_pcg::Pcg64,
    crossovers: &mut Vec<LargeSignedInteger>,
) {
    crossovers.clear();
    crossovers.push(0);
    for _ in 0..num_crossovers {
        let pos = crossover_position.sample(rng);
        assert!(pos > 0 && pos < genome_length);
        crossovers.push(pos);
    }
    crossovers.sort_unstable();
    crossovers.push(genome_length);
}

fn make_crossover_position_distribution(
    genome_length: LargeSignedInteger,
) -> rand_distr::Uniform<LargeSignedInteger> {
    assert!(genome_length > 0);
    rand_distr::Uniform::new(1, genome_length)
}

pub fn evolve<N: EvolveAncestry>(
    seed: u64,
    parameters: Parameters,
    population: &mut N,
) -> Result<(), Box<dyn Error>> {
    let rng = Rc::new(RefCell::<rand_pcg::Pcg64>::new(
        rand_pcg::Pcg64::seed_from_u64(seed),
    ));
    let mut death = Death::new(parameters.death_probability, rng.clone());

    population.setup(parameters.nsteps);

    let popsize = population.current_population_size();

    let parent_picker = rand_distr::Uniform::new(0, popsize);
    let num_crossovers = rand_distr::Poisson::new(parameters.mean_num_crossovers)?;
    let crossover_position = make_crossover_position_distribution(population.genome_length());
    let mendel = rand_distr::Bernoulli::new(0.5).unwrap();
    let mut transmissions: Vec<TransmittedSegment> = vec![];
    let mut crossovers: Vec<LargeSignedInteger> = vec![];
    for step in 1..parameters.nsteps + 1 {
        let nreplacements = population.generate_deaths(&mut death);
        let mut mut_borrowed_rng = rng.borrow_mut();
        for _ in 0..nreplacements {
            let mut p1 = parent_picker.sample(mut_borrowed_rng.deref_mut());
            let mut p2 = parent_picker.sample(mut_borrowed_rng.deref_mut());
            if mendel.sample(mut_borrowed_rng.deref_mut()) {
                std::mem::swap(&mut p1, &mut p2);
            }
            // let n = num_crossovers.sample(mut_borrowed_rng.deref_mut()) as u64;
            let n = 1;
            generate_crossover_positions(
                population.genome_length(),
                n,
                &crossover_position,
                mut_borrowed_rng.deref_mut(),
                &mut crossovers,
            );
            fill_transmissions(p1, p2, &crossovers, &mut transmissions);
            population.record_birth(step, parameters.nsteps, &transmissions)?;
        }
        population.simplify(step)?;
    }

    population.finish(parameters.nsteps)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rng(seed: u64) -> rand_pcg::Pcg64 {
        rand_pcg::Pcg64::seed_from_u64(seed)
    }

    fn make_transmission(
        left: LargeSignedInteger,
        right: LargeSignedInteger,
        parent: usize,
    ) -> TransmittedSegment {
        TransmittedSegment::new(left, right, parent)
    }

    macro_rules! validate_transmissions {
        ($expected: ident, $got: ident) => {
            assert_eq!($expected.len(), $got.len());

            for (i, j) in $expected.iter().zip($got.iter()) {
                assert_eq!(*i, *j);
            }
        };
    }

    #[test]
    fn test_generate_crossover_positions() {
        let mut rng = make_rng(101);
        let genome_length = 10_i64;
        let mut crossovers = vec![];
        let crossover_position = make_crossover_position_distribution(genome_length);
        for n in 0..10_u64 {
            generate_crossover_positions(
                genome_length,
                n,
                &crossover_position,
                &mut rng,
                &mut crossovers,
            );
            assert_eq!(crossovers.len() as u64, n + 2);
            let sorted = crossovers.windows(2).all(|w| w[0] <= w[1]);
            assert!(sorted);
        }
    }

    #[test]
    fn test_fill_transmissions() {
        let p1 = 0_usize;
        let p2 = p1 + 1;
        let genome_length = 100_i64;
        let mut transmissions = vec![];

        {
            let crossovers = vec![0, genome_length];
            let expected = vec![make_transmission(0, genome_length, p1)];
            fill_transmissions(p1, p2, &crossovers, &mut transmissions);
            validate_transmissions!(expected, transmissions);
        }

        {
            let crossovers = vec![0, 1, 3, genome_length];
            let expected = vec![
                make_transmission(0, 1, p1),
                make_transmission(1, 3, p2),
                make_transmission(3, genome_length, p1),
            ];
            fill_transmissions(p1, p2, &crossovers, &mut transmissions);
            validate_transmissions!(expected, transmissions);
        }

        {
            let crossovers = vec![0, 1, 3, 3, genome_length];
            let expected = vec![
                make_transmission(0, 1, p1),
                make_transmission(1, genome_length, p2),
            ];
            fill_transmissions(p1, p2, &crossovers, &mut transmissions);
            validate_transmissions!(expected, transmissions);
        }

        {
            let crossovers = vec![0, 1, 3, 3, 3, genome_length];
            let expected = vec![
                make_transmission(0, 1, p1),
                make_transmission(1, 3, p2),
                make_transmission(3, genome_length, p1),
            ];
            fill_transmissions(p1, p2, &crossovers, &mut transmissions);
            validate_transmissions!(expected, transmissions);
        }

        {
            let crossovers = vec![0, 1, 1, 3, 3, 3, genome_length];
            let expected = vec![
                make_transmission(0, 3, p1),
                make_transmission(3, genome_length, p2),
            ];
            fill_transmissions(p1, p2, &crossovers, &mut transmissions);
            validate_transmissions!(expected, transmissions);
        }

        {
            let crossovers = vec![0, 1, 1, 3, 3, 3, 7, genome_length];
            let expected = vec![
                make_transmission(0, 3, p1),
                make_transmission(3, 7, p2),
                make_transmission(7, genome_length, p1),
            ];
            fill_transmissions(p1, p2, &crossovers, &mut transmissions);
            validate_transmissions!(expected, transmissions);
        }
    }
}
