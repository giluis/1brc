use ahash::{AHashMap, AHasher, HashMap, RandomState};
use generate::CITIES;
use itertools::{Itertools, intersperse};
use rand::{prelude::Rng, seq::SliceRandom};
use std::io::prelude::Write;
use std::io::stdout;
use std::time::Instant;
use std::{
    fs::read,
    hash::{BuildHasher, Hasher},
};

use crate::generate::generate_file;

mod generate;
mod hashfunc;

#[derive(Clone, Copy)]
struct Record {
    min: u16,
    max: u16,
    sum: u32,
    count: u32,
}

impl Record {
    fn empty() -> Self {
        Record {
            min: u16::MAX,
            max: u16::MIN,
            sum: 0,
            count: 0,
        }
    }

    // TODO: check inline always
    // TODO: check value as (u8,u8) instead of u16
    fn process(&mut self, value: u16) {
        // TODO: unchecked operations
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value as u32;
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

type Fastf = u16;

/**
 * Returns shift necessary for next_starting_point
 */
fn fast_hash<'a>(
    s: &'a [u8],
    start: usize,
    // hasher: &mut ahash::AHasher,
    measurements: &mut [(Option<&'a [u8]>, Record)],
) -> usize {
    let mut hash = 0xcbf29ce484222325u64;
    // TODO: unchecked indexing
    let mut i = start;
    // TODO: check from the back instead of the front of the string
    // Saves this while loop, but makes check more complicated
    // Might work for longer string names
    while s[i] != b';' {
        hash ^= s[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    // hasher.write(&s[start..i - 1]);
    // let _ = dbg!(String::from_utf8(s[start..i].into()));
    let idx = (hash % 20) as usize;
    if measurements[idx].0.is_none() {
        measurements[idx].0 = Some(&s[start..i])
    }

    i += 1;
    let is_negative = s[i] == b'-';
    if is_negative {
        i += 1
    }

    let mut value = 0;
    // TODO: Check loop unrolled instead of *100 , *10
    if s[i + 1] == b'.' {
        // handle a.b
        value = (s[i] - 48) as u16 * 10;
        i += 2;
        value += (s[i] - 48) as u16;
    } else if s[i + 2] == b'.' {
        // handle ab.c
        value = (s[i] - 48) as u16 * 100;
        i += 1; 
        value += (s[i] - 48) as u16 * 10;
        i += 2;
        value += (s[i] - 48) as u16;
    } 

    if is_negative {
        value = 999 - value;
    } else {
        value += 999;
    }

    // TODO: check pass by copy instead of reference
    measurements[idx].1.process(value);
    // skip paragraph
    (i + 2) - start
}

fn improved_parsing(size: usize) {
    let source = std::fs::read(format!("../measurements_{size}.txt")).unwrap();
    let mut start = 0;
    let empty = "EMPTY";
    // TODO: check transmute [0u64;20_000] here (Option<&str> takes as much space as &str)
    let mut measurements = [(None, Record::empty()); 20];
    // let mut hasher = RandomState::new().build_hasher();
    while start < source.len() {
        start += fast_hash(&source, start,  &mut measurements);
    }
    let mut buf = Vec::with_capacity(20 * (14 + 15));
    measurements.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    measurements.into_iter().for_each(|(city_name, record)| {
        if let Some(name) = city_name {
            dbg!(String::from_utf8(name.into()));
            write_city(&mut buf, name, record);
        }
    });
    let len = buf.len();
    buf[len - 1] = b'}';
    stdout().lock().write_all(&buf).unwrap();
}

fn write_city(
    buff: &mut Vec<u8>,
    city_name: &[u8],
// TODO: check pass by reference
    Record {
        min,
        max,
        sum,
        count,
    }: Record,
) {
    buff.extend_from_slice(city_name);
    buff.push(b'=');
    write_value(buff, min);
    write_value(buff, max);
    write_mean(buff, sum, count);
    buff.push(b',')
}

fn write_value(buff: &mut Vec<u8>, value: u16) {
    // TODO: unchecked arithmetic
    buff.push(48 + (value / 10) as u8);
    // write_number(buff, value - 999);
    buff.push(b'.');
    // TODO: unchecked arithmetic
    buff.push(48 + (value % 10) as u8);
    buff.push(b'/');
}


fn write_n(buff: &mut Vec<u8>, mut value: u16) {
    if value < 999 {
        // TODO: check subtraction here
        let real_value = 999 - value;
        real_value % 10
        buff.push(b'-');
    }


}

// TODO: check inline always
fn write_number(buff: &mut Vec<u8>, mut value: u8) {
    if value >= 10  {
        buff.push(48 + value / 10) ;
    }
    buff.push(48 + value % 10);
}

fn write_mean(buff: &mut Vec<u8>, sum: u32, count: u32) {
    let mean = sum / count * 10 - 999;
    // TODO: unchecked arithmetic
    let integer_part =  mean  as u8 / 100;
    write_number(buff, integer_part);
    // TODO: unchecked arithmetic
    let frac_part = mean as u8 % 100;
    buff.push(b'.');
    write_number(buff, frac_part);
}

fn check<F: FnOnce(usize)>(size: usize, solution: F) -> bool {
    solution(size);
    fn read_hashmap(string: &str) -> AHashMap<&str, [f32; 3]> {
        let mut result = AHashMap::new();
        for l in string.split(',') {
            println!("{l}");
            let (city_name, values_str) = match l.trim().split_once('=') {
                Some(a) => a,
                None => break,
            };
            let mut values = [0.0; 3];
            let mut idx = 0;
            for v in values_str.split('/') {
                values[idx] = v.parse().unwrap();
                idx += 1;
            }

            if idx != 3 {
                panic!("Reading went wrong");
            }
            result.insert(city_name, values);
        }
        result
    }

    let expected = std::fs::read_to_string(format!("./expected_{size}.txt")).unwrap();
    let result = std::fs::read_to_string(format!("./result_{size}.txt")).unwrap();
    let expected = read_hashmap(&expected);
    let result = read_hashmap(&result);
    expected == result
}

fn baseline(size: usize) {
    let a = std::fs::read_to_string(format!("../measurements_{size}.txt")).unwrap();
    let mut measurements = std::collections::HashMap::new();
    for l in a.lines() {
        let (city_name, value) = l.split_once(';').unwrap();
        let a: f32 = value.parse().unwrap();
        let entry = measurements.entry(city_name).or_insert((0.0, 0.0, 0.0, 0));
        entry.0 = a.min(entry.0);
        entry.1 = a.max(entry.1);
        entry.2 += a;
        entry.3 += 1;
    }
    let m: Vec<_> = measurements
        .iter()
        .sorted_by(|(a, _), (b, _)| a.cmp(b))
        .collect();

    let mut s = "{".to_owned();
    measurements
        .iter()
        .for_each(|(city_name, (min, max, sum, count))| {
            s += &format!(
                "{city_name}={}/{}/{},",
                min,
                max,
                (10.0 * (sum / *count as f32)).ceil() / 10.0
            );
        });
    s += "}";
    std::fs::write(format!("./expected_{size}.txt"), s).unwrap();
}

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash % 10_000
}

fn main() {
    // let a = check(10_000, improved_parsing);
    // improved_parsing(1_000_000_000);
    // generate_file(100);
    // let mut hasher = AHasher::default();

    // improved_parsing(100);

    println!("{}", 3_u16/10_u16)
    // let avg = (0..5).map(|_| {
    //     let timer = Instant::now();
    //     let mut n:u32 = 0;
    //     for i in 0..10000{
    //         n += i * 2;
    //         if n > 50{
    //             n /= 100;

    //         }
    //     }
    //     println!("result: {n}");
    //     timer.elapsed().as_micros() 
    // }).sum::<u128>() / 5;

    // println!("took {}\n\n", avg);

    // let avg = (0..5).map(|_| {
    //     let timer = Instant::now();
    //     let mut n:i32 = 0;
    //     for i in 0..10000{
    //         n += i * 2;
    //         if n > 50{
    //             n /= 100;
    //         }
    //     }
    //     println!("result: {n}");
    //     timer.elapsed().as_micros() 
    // }).sum::<u128>() / 5;

    // println!("took {}\n\n", avg);
    // println!("{}", (0 + 999 + 1998) / 30);
    // println!("{}", (0 + 999 + 1998) / 3 % 10);
}

/* Generation */
