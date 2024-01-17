<<<<<<< HEAD
use std::time::Instant;
=======
use ahash::{AHasher, HashMap, RandomState};
>>>>>>> 9780846... drop custom hash, use ahasher
use generate::CITIES;
use rand::{prelude::Rng, seq::SliceRandom};
use std::{
    fs::read,
    hash::{BuildHasher, Hasher},
    time::Instant,
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
        i+=4;
        ((s[i] - b'0'), (s[i + 2] - b'0'))
    } else {
        i+=5;
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
    generate_file(10_000);
    generate_file(10_000_000);
    generate_file(1_000_000_000);
    // let timer = Instant::now();
    // let a = std::fs::read("../measurements_10000.txt").unwrap();
    // println!("Took {} ms to read file", timer.elapsed().as_millis());
    // const NUM_THREADS: usize = 8;
    // let mut start = 0;
    // let mut measurements = [((0, 0), (0, 0), (0, 0), 0); 10_000];
    // let mut hasher = RandomState::new().build_hasher();
    // let batch_size = a.len() / NUM_THREADS;
    // let timer = Instant::now();

    // while start < a.len() {
    //     start += fast_hash(&a, &mut hasher, &mut measurements);
    //     // println!("{}", a[start - 1] == b'\n');
    // }
    // println!("Took {:?} ms finnish", timer.elapsed().as_millis());
    // println!("{:?}", &measurements[0..1000]);
    // let mut starting_points = [0; NUM_THREADS];
    // let timer = Instant::now();
    // for i in 1..NUM_THREADS {
    //     let mut s_idx = starting_points[i - 1] + batch_size;
    //     while a[s_idx] != b'\n' {
    //         s_idx += 1;
    //     }
    //     s_idx += 1;
    //     starting_points[i] = s_idx;
    // }
}

/* Generation */
