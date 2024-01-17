use ahash::{AHasher, HashMap, RandomState};
use generate::CITIES;
use rand::{prelude::Rng, seq::SliceRandom};
use std::time::Instant;
use std::{
    fs::read,
    hash::{BuildHasher, Hasher},
};

use crate::generate::generate_file;

mod generate;
mod hashfunc;

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

type Fastf = (u8, u8);

/**
 * Returns shift necessary for next_starting_point
 */
fn fast_hash(
    s: &[u8],
    hasher: &mut ahash::AHasher,
    measurements: &mut [(Fastf, Fastf, Fastf, usize)],
) -> usize {
    let mut i = 0;
    while s[i] != b';' {
        i += 1;
    }
    hasher.write(&s[0..i - 1]);
    i += 1;
    let is_negative = s[i] == b'-';
    if is_negative {
        i += 1
    }
    let n = if s[i + 1] == b'.' {
        i += 4;
        ((s[i] - b'0'), (s[i + 2] - b'0'))
    } else {
        i += 5;
        ((s[i] - b'0') * 10 + (s[i + 1] - b'0'), (s[i + 3] - b'0'))
    };
    // skip the '.'
    let idx = (hasher.finish() % 10_000) as usize;
    let t = &mut measurements[idx];
    t.0 = t.0.min(n);
    t.1 = t.1.min(n);
    t.2 .0 += n.0;
    t.2 .1 += n.1;
    t.3 += 1;
    i
}

fn main() {
    let timer = Instant::now();
    let a = std::fs::read("../measurements_1000000000.txt").unwrap();
    println!("Took {} ms to read file", timer.elapsed().as_millis());
    let mut start = 0;
    let mut measurements = [((0, 0), (0, 0), (0, 0), 0); 10_000];
    let mut hasher = RandomState::new().build_hasher();
    let timer = Instant::now();
    while start < a.len() {
        start += fast_hash(&a, &mut hasher, &mut measurements);
        // println!("{}", a[start - 1] == b'\n');
    }
    println!("Took {:?} ms finnish", timer.elapsed().as_millis());
}

/* Generation */
