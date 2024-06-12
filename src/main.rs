#![allow(clippy::type_complexity)]
#![feature(let_chains)]
#![feature(maybe_uninit_uninit_array)]
#![feature(array_windows)]
#![feature(generic_const_exprs)]
#![feature(generic_arg_infer)]

use memmap2::MmapOptions;
use record::Record;
use std::{
    io::{stdout, Write}, mem::MaybeUninit, slice::ArrayWindows, sync::{Arc, Mutex, RwLock}, thread, time::Instant
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

const NUM_BUCKETS: usize = 3 * 10_000;

#[derive(Debug)]
struct Measurements<'a>([RwLock<Record<'a>>; NUM_BUCKETS]);

impl<'a> Measurements<'a> {
    fn new() -> Self {
        // From ChatGPT
        // SAFETY: This is safe because MaybeUninit<T> does not require initialization.
        let mut array: [MaybeUninit<RwLock<Record>>; NUM_BUCKETS] =
            unsafe { MaybeUninit::uninit().assume_init() };

        // Initialize each element of the array safely.
        for elem in &mut array[..] {
            *elem = MaybeUninit::new(RwLock::new(Record::empty()));
        }

        // SAFETY: All elements of the array are initialized, so this is now safe.
        let initialized_array: [RwLock<Record>; NUM_BUCKETS] =
            unsafe { std::mem::transmute(array) };
        Self(initialized_array)
    }

    const fn num_buckets() -> usize {
        NUM_BUCKETS
    }

    #[inline(always)]
    fn process_at(&self, mut hashed_idx: usize, city_name: &'a [u8], value: u16) {
        // TODO: get unchecked
        let mut r = self.0[hashed_idx].write().expect("Lock was poisoned");
        loop {
            match &r.name {
                Some(n) if *n == city_name => r.process(value),
                None => {
                    *r = Record::new_with_initial(city_name, value);
                    break;
                }
                Some(_) => {
                    hashed_idx += 1;
                    hashed_idx %= NUM_BUCKETS
                }
            }
        }
    }

    fn as_sorted(self) -> [Record<'a>; NUM_BUCKETS] {
        self.0.map(|r| RwLock::into_inner(r).unwrap())
    }
}

/**
 * Returns shift necessary for next_starting_point
 */
fn fast_hash<'a>(s: &'a [u8], start: usize, measurements: &Arc<Measurements<'a>>) -> usize {
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

    measurements.process_at(hashed_idx, &s[start..=name_end], value);
    // skip paragraph
    (i + 2) - start
}

// fn chunks<const NUM_CHUNKS: usize>(source: &[u8]) -> ArrayWindows<'_,usize, NUM_CHUNKS>{
//     let mut chunk_borders = [0; {NUM_CHUNKS + 1}];
//     * chunk_borders.last_mut().unwrap() = source.len();
//     chunk_borders[1..(NUM_CHUNKS - 1)]
//         .iter_mut()
//         .enumerate()
//         .for_each(|(i, e)| {
//             *e = i * (source.len() / NUM_CHUNKS);
//             while source[*e] != b'\n' {
//                 *e += 1;
//             }
//         });
//      chunk_borders.array_windows()
// }

fn improved_parsing() {
    let timer = Instant::now();
    let source = std::fs::File::open("../measurements_100.txt").unwrap();
    let file_len = source.metadata().unwrap().len() as usize;
    let source = unsafe { MmapOptions::new().map(&source).unwrap() };
    let a = &source;



    println!("Took {:?} to read file", timer.elapsed());
    const NUM_CORES: usize = 8;
    let measurements = Arc::new(Measurements::new());
    dbg!(source.len());
    thread::scope(|s| {
        let sref = *source;
        for chunk in chunk_borders {
            let ptr = Arc::clone(&measurements);
            // let thread_source = Arc::clone(&source);
            s.spawn(move || {
                let [mut start, end] = chunk;
                let end = *end;
                while start < end {
                    start += fast_hash(sref, start, &ptr);
                }
            });

        }
        
    });

    let measurements = Arc::try_unwrap(measurements).unwrap().as_sorted();

    let mut buf = Vec::with_capacity(NUM_BUCKETS * (14 + 15));

    // measurements_flat.sort_unstable_by(|a,b| a.cmp(b, &*source));
    measurements.into_iter().for_each(|record| {
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
    record: Record,
) {
    let Record {
        min,
        max,
        sum,
        count,
        ..
    } = record;
    buff.extend_from_slice(record.name.unwrap());
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
