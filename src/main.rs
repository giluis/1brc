#![allow(clippy::type_complexity)]
#![feature(let_chains)]
#![feature(maybe_uninit_uninit_array)]

use memmap2::MmapOptions;
use record::Record;
use std::{
    io::{stdout, Write},
    mem::MaybeUninit,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::Instant,
};

#[allow(dead_code)]
mod baseline;

#[allow(dead_code)]
mod generate;
mod record;

/**
 * Idx and len
 */
#[inline(always)]
fn hash(s: &[u8], start: usize) -> (usize, usize) {
    let mut hash = 0xcbf29ce484222325u64;
    // TODO: unchecked indexing
    let mut i = start;
    // TODO: check from the back instead of the front of the string
    // Saves this while loop, but makes check more complicated
    // Might work for longer string names
    while unsafe { *s.get_unchecked(i) } != b';' {
        hash ^= unsafe { *s.get_unchecked(i) } as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1
    }
    let idx = (hash % Measurements::num_buckets() as u64) as usize;
    (idx, i)
}

#[derive(Debug)]
struct Measurements([RwLock<Vec<Record>>; NUM_BUCKETS]);

unsafe impl Sync for Measurements{}
unsafe impl Send for Measurements{}

const NUM_BUCKETS: usize = 3 * 10_000;

impl Measurements {
    fn new() -> Self {
        // SAFETY: This is safe because MaybeUninit<T> does not require initialization.
        let mut array: [MaybeUninit<RwLock<Vec<Record>>>; NUM_BUCKETS] =
            unsafe { MaybeUninit::uninit().assume_init() };

        // Initialize each element of the array safely.
        for elem in &mut array[..] {
            *elem = MaybeUninit::new(RwLock::new(vec![]));
        }

        // SAFETY: All elements of the array are initialized, so this is now safe.
        let initialized_array: [RwLock<Vec<Record>>; NUM_BUCKETS] =
            unsafe { std::mem::transmute(array) };
        Self(initialized_array)
    }

    const fn num_buckets() -> usize {
        NUM_BUCKETS
    }

    #[inline(always)]
    fn process_at(&mut self, hashed_idx: usize, city_name: (usize, usize), value: u16) {
        // TODO: get unchecked
        let mut values_for_hash = unsafe { self.0.get_unchecked(hashed_idx) }.write().unwrap();
        for s in values_for_hash.iter_mut() {
            if s.name == city_name {
                s.process(value);
                return;
            }
        }
        values_for_hash.push(Record::new_with_initial(city_name, value));
    }
}

/**
 * Returns shift necessary for next_starting_point
 */
fn fast_hash<'a>(s: &'a [u8], start: usize, measurements: Arc<Measurements>) -> usize {
    let (hashed_idx, name_end) = hash(&s, start);

    // skip ';'
    let mut i = name_end + 1;

    let is_negative = unsafe { *s.get_unchecked(i) } == b'-';
    if is_negative {
        // skip '-', if it exists
        i += 1
    }

    let mut value = 0;
    // TODO: Check loop unrolled instead of *100 , *10
    if unsafe { *s.get_unchecked(i + 1) } == b'.' {
        // handle a.b
        value = (unsafe { *s.get_unchecked(i) } - 48) as u16 * 10;
        i += 2;
        value += (unsafe { *s.get_unchecked(i) } - 48) as u16;
    } else if unsafe { *s.get_unchecked(i + 2) } == b'.' {
        // handle ab.c
        value = (unsafe { *s.get_unchecked(i) } - 48) as u16 * 100;
        i += 1;
        value += (unsafe { *s.get_unchecked(i) } - 48) as u16 * 10;
        i += 2;
        value += (unsafe { *s.get_unchecked(i) } - 48) as u16;
    }

    if is_negative {
        value = 999 - value;
    } else {
        value += 999;
    }

    measurements.process_at(hashed_idx, (start, name_end), value);
    // skip paragraph
    (i + 2) - start
}


fn improved_parsing() {
    let timer = Instant::now();
    let source = std::fs::File::open("../measurements_1000000000.txt").unwrap();
    let file_len = source.metadata().unwrap().len() as usize;
    let source = Arc::new(unsafe { MmapOptions::new().map(&source).unwrap() });

    println!("Took {:?} to read file", timer.elapsed());
    const NUM_CORES: usize = 8;
    let mut chunks = [(0, 0); NUM_CORES];
    chunks[0] = (0, file_len / NUM_CORES);
    for i in 1..(NUM_CORES - 1) {
        chunks[i].0 = chunks[i - 1].1 + 1;
        chunks[i].1 = chunks[i].0 + file_len / NUM_CORES;
    }

    chunks[NUM_CORES - 1] = (chunks[NUM_CORES - 2].1 + 1, file_len);

    let measurements = Arc::new(Measurements::new());

    let mut handles = Vec::new();

    for core_idx in 0..NUM_CORES {
        let ptr = Arc::clone(&measurements);
        let thread_source = Arc::clone(&source);
        let handle = thread::spawn(move || unsafe {
            let (mut start, end) = chunks[core_idx];
            while start < end {

                start += fast_hash(thread_source.as_ref(), start, ptr );
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    let measuerements = Arc::try_unwrap(measurements).unwrap();

    let mut buf = Vec::with_capacity(NUM_BUCKETS * (14 + 15));
    let mut measurements_flat: Vec<_> = measurements
        .0
        .into_iter()
        .flat_map(|v| v.into_inner().unwrap().into_iter())
        .collect();

    measurements_flat.sort_unstable_by(|a,b| a.cmp(b, &*source));
    measurements_flat.into_iter().for_each(|record| {
        write_city(&mut buf, record, &*source);
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
    record: Record,
    source: &[u8], 
) { 
    
    let Record {
        min,
        max,
        sum,
        count,
        ..
    } =record;
    buff.extend_from_slice(record.name(source));
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
