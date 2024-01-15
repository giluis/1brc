use std::{time::Instant, slice::range};
use generate::CITIES;
use rand::prelude::Rng;

use crate::generate::generate_file;

mod generate;
mod hashfunc;


struct Analysis {
    min: f32,
    max: f32,
    sum: f32,
    count: usize,
}

impl std::fmt::Debug for Analysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { min, max, .. } = self;
        write!(f, "(min: {min}, max: {max}, mean: {:.1})", self.mean())
    }
}

#[allow(dead_code)]
impl Analysis {
    fn mean(&self) -> f32 {
        self.sum / self.count as f32
    }

    fn new() -> Self {
        Self {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
            sum: 0.0,
            count: 0,
        }
    }

    fn process(&mut self, value: f32) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.count += 1;
    }
}

enum Enum {
    First(usize),
    Second(usize),
}

fn random_cities() -> Vec<&'static str> {
    let mut rng = rand::thread_rng();
    let mut result = vec![];
    for _ in 0..10_000 {
        result.push(CITIES[rng.gen_range(0..CITIES.len())])
    }
    result
}

fn main() {
    let mut hashmap = ahash::AHashMap::<&str, (usize, usize, usize, usize)> ::new();
    let keys = random_cities();
    for k in keys {
        hashmap.insert(k, (0,0,0,0));
    }
    itertools::un CITIES.map(|c|c.chars().first().unwrap())


    let timer = Instant::now();
    for _ in 0..10000  {
        // get random value in hashmap


        // get random value from array

    }
    let elapsed = timer.elapsed(); 
}

/* Generation */

