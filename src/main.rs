#![allow(clippy::type_complexity)]
#![feature(let_chains)]
use itertools::Itertools;
use memmap2::MmapOptions;
use record::Record;
use std::{io::{stdout, Write}, time::Instant};

mod baseline;
#[allow(dead_code)]
mod generate;
mod record;

const NUM_CITIES: usize = 10_000;
const CITIES_INFO_SIZE: usize = NUM_CITIES * 6;

fn collision_hash(s: &[u8]) -> usize {
    let mut hash = 0xcbf29ce484222325u64;
    for c in s {
        hash ^= *c as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash % (NUM_CITIES * 6) as u64) as usize
}
/**
 * Idx and len
 */
fn hash(s: &[u8], start: usize) -> (usize, usize) {
    let mut hash = 0xcbf29ce484222325u64;
    // TODO: unchecked indexing
    let mut i = start;
    // TODO: check from the back instead of the front of the string
    // Saves this while loop, but makes check more complicated
    // Might work for longer string names
    while unsafe{*s.get_unchecked(i)} != b';' {
        hash ^= unsafe{*s.get_unchecked(i)} as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1
    }
    let  idx = (hash % Measurements::num_buckets() as u64) as usize;
    (idx, i)
}
struct MeasurementsGeneric<'a, const NUM_BUCKETS: usize, const BUCKET_DEPTH: usize>(
    [[Option<Record<'a>>; BUCKET_DEPTH]; NUM_BUCKETS],
);

impl<'a, const NUM_BUCKETS: usize, const BUCKET_DEPTH: usize>
    MeasurementsGeneric<'a, NUM_BUCKETS, BUCKET_DEPTH>
{
    fn new() -> Self {
        Self([[None; BUCKET_DEPTH]; NUM_BUCKETS])
    }

    const fn total_size() -> usize {
        NUM_BUCKETS * BUCKET_DEPTH
    }

    const fn num_buckets() -> usize {
        NUM_BUCKETS
    }

    fn process_at(&mut self, hashed_idx: usize, city_name: &'a [u8], value: u16) {
        // TODO: get unchecked
        let values_for_hash = self.0.get_mut(hashed_idx).unwrap();
        for stored_city in values_for_hash.iter_mut() {
            match stored_city {
                Some(sc) if sc.name == city_name => sc.process(value),
                None => {
                    * stored_city = Some(Record::new_with_initial(city_name,value)); 
                    return;
                }
                _ => continue,
            }
        }
        unreachable!("Loop should have gotten to an empty bucket");
    }

}

/**
 * Returns shift necessary for next_starting_point
 */
fn fast_hash<'a>(s: &'a [u8], start: usize, measurements: &mut Measurements<'a>) -> usize {
    let (hashed_idx, name_end) = hash(s, start);

    // skip ';'
    let mut i = name_end + 1;

    let is_negative = unsafe{*s.get_unchecked(i)} == b'-';
    if is_negative {
        // skip '-', if it exists
        i += 1
    }

    let mut value = 0;
    // TODO: Check loop unrolled instead of *100 , *10
    if unsafe{*s.get_unchecked(i + 1)} == b'.' {
        // handle a.b
        value = (unsafe{*s.get_unchecked(i)} - 48) as u16 * 10;
        i += 2;
        value += (unsafe{*s.get_unchecked(i)} - 48) as u16;
    } else if unsafe{*s.get_unchecked(i + 2)} == b'.' {
        // handle ab.c
        value = (unsafe{*s.get_unchecked(i)} - 48) as u16 * 100;
        i += 1;
        value += (unsafe{*s.get_unchecked(i)} - 48) as u16 * 10;
        i += 2;
        value += (unsafe{*s.get_unchecked(i)} - 48) as u16;
    }

    if is_negative {
        value = 999 - value;
    } else {
        value += 999;
    }

    measurements.process_at(hashed_idx, &s[start..name_end], value);
    // skip paragraph
    (i + 2) - start
}

type Measurements<'a> = MeasurementsGeneric<'a, 10_000, 3>;

fn improved_parsing() {
    let timer = Instant::now();
    let source = std::fs::File::open("../measurements_1000000000.txt").unwrap();
    let file_len = source.metadata().unwrap().len() as usize;
    let source = unsafe { MmapOptions::new().map(&source).unwrap() };
    // let source = std::fs::read(format!("../measurements_{size}.txt")).unwrap();
    // let file_len = source.len();
    println!("Took {:?} to read file", timer.elapsed());
    let mut start = 0;
    // TODO: check transmute [0u64;20_000] here (Option<&str> takes as much space as &str)
    let mut measurements = Measurements::new();
    // let mut hasher = RandomState::new().build_hasher();
    while start < file_len {
        start += fast_hash(&source, start, &mut measurements);
    }

    let mut buf = Vec::with_capacity(Measurements::total_size() * (14 + 15));

    let mut measurements_flat: [Option<Record>; Measurements::total_size()] = unsafe {
        // Using `std::mem::transmute` to perform the conversion.
        // Safety: This is safe because the source and target types have the same total size,
        // and `u8` elements do not have alignment requirements or invalid states.
        std::mem::transmute(measurements)
    };

    measurements_flat.sort_unstable();
    measurements_flat
        .into_iter()
        .flatten()
        .for_each(|record| {
            write_city(&mut buf, record);
        });

    // println!("{:?}", measurements_flat.iter().rev());

    let len = buf.len();
    buf[len - 1] = b'}';
    stdout().lock().write_all(&buf).unwrap();
    println!("\nTook {:?} to process", timer.elapsed());
}

fn write_city(
    buff: &mut Vec<u8>,
    // TODO: check pass by reference
    Record {
        name,
        min,
        max,
        sum,
        count,
    }: Record,
) {
    buff.extend_from_slice(name);
    buff.push(b'=');
    write_n(buff, min);
    buff.push(b'/');
    write_n(buff, max);
    buff.push(b'/');
    write_n(buff, mean(sum, count));
    buff.push(b',')
}

fn mean(sum: u32, count: u32) -> u16 {
    let mean = (sum / count) as u16;
    if sum % count == 0 {
        mean
    } else if mean > 999 {
        mean + 1
    } else {
        mean
    }
}

fn write_n(buff: &mut Vec<u8>, value: u16) {
    // TODO: check mutating value instead of assigning to real value
    let mut real_value = if value < 999 {
        // TODO: check subtraction here
        buff.push(b'-');
        999 - value
    } else {
        value - 999
    };

    if real_value >= 100 {
        buff.push(48 + (real_value / 100) as u8);
        real_value %= 100;
    }

    buff.push(48 + (real_value / 10) as u8);
    buff.push(b'.');
    buff.push(48 + (real_value % 10) as u8);
}

fn main() {
    improved_parsing();
    // println!("File has been generated");
    // let source = std::fs::read_to_string("./measurements_1000000000.txt").unwrap();
    // let a: AHashSet<&str> = source.lines().map(|l|l.split_once(';').unwrap().0).collect();
    // println!("Num cities {}", a.len());
}

#[cfg(test)]
mod tests {

    use crate::{mean, write_n};

    #[test]
    fn write_n_test() {
        let mut i = -999;
        while i <= 999 {
            // println!("{i}");
            let mut buff = vec![];
            write_n(&mut buff, (i + 999) as u16);
            let result: f32 = String::from_utf8(buff).unwrap().parse().unwrap();
            assert_eq!((result * 10.0).round() / 10.0, (i as f32).round() / 10.0);
            i += 1;
        }
    }

    #[test]
    fn write_city_test() {
        // let city_name = "Porto";
        // let record = Record {
        //     max: (16) + 999,          // 1015
        //     min: (-964 + 999) as u16, // 35
        //     count: 3,
        //     sum: 1015 + 35 + 88,
        // };

        // let expected = format!("{city_name}={}/{}/{},", record.min, record.max, 62.0);
        let mut i = -999;
        while i <= 999 {
            // println!("{i}");
            let mut buff = vec![];
            write_n(&mut buff, (i + 999) as u16);
            let result: f32 = String::from_utf8(buff).unwrap().parse().unwrap();
            assert_eq!((result * 10.0).round() / 10.0, (i as f32).round() / 10.0);
            i += 1;
        }
    }

    #[test]
    fn test_mean() {
        // let inputs_expected = [([1035, 998, 1001], 13 + 999)];

        // let mean = mean(1035 + 998 + 1001, 3);
        // assert_eq!(mean, 13 + 999);

        let mean = mean(999 + 1338 + 999, 3);
        assert_eq!(mean, 339)
    }
}

/* Generation */
