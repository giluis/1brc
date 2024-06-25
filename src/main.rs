#![feature(ascii_char)]
#![allow(clippy::type_complexity)]
#![feature(let_chains)]
#![feature(maybe_uninit_uninit_array)]
#![feature(array_windows)]
#![feature(generic_const_exprs)]
#![feature(generic_arg_infer)]

use memmap2::MmapOptions;
use record::Record;
use std::{
    io::{stdout, Write},
    mem::MaybeUninit,
    sync::RwLock,
    thread,
    time::Instant,
};

#[allow(dead_code)]
mod baseline;

#[allow(dead_code)]
mod generate;
mod record;

const INITIAL_HASH: u64 = 0xcbf29ce484222325u64;
const HASH_WRAP_MUL: u64 = 0x100000001b3;

/**
 * Returns (hashed_idx, len),
 * - hashed_idx is the predicted location of the city starting at `start` in s
 * - len is the length of this city name
 */
#[inline(always)]
fn hash_find(s: &[u8], start: usize) -> (usize, usize) {
    let mut hash = INITIAL_HASH;
    // TODO: unchecked indexing
    let mut city_name_end = start;
    // TODO: check from the back instead of the front of the string
    // Saves this while loop, but makes check more complicated
    // Might work for longer string names
    while s[city_name_end] != b';' {
        hash ^= s[city_name_end] as u64;
        hash = hash.wrapping_mul(HASH_WRAP_MUL);
        city_name_end += 1
    }
    let hashed_idx = (hash % Measurements::num_buckets() as u64) as usize;
    // set i before semi_colon
    (hashed_idx, city_name_end - 1)
}

const NUM_BUCKETS: usize = 22000;

#[derive(Clone)]
struct Measurements<'a>(Vec<Record<'a>>);

struct MeasurementsIterator {
    idx: usize,
}

impl<'a> std::fmt::Debug for Measurements<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = "{".to_owned();
        for r in self.0.iter().filter(|r| r.name.is_some()) {
            result += &r.to_string();
        }
        result += "}";
        write!(f, "{result}")
    }
}

impl<'a> Measurements<'a> {
    fn new() -> Self {
        // From ChatGPT
        // SAFETY: This is safe because MaybeUninit<T> does not require initialization.
        // let mut array: [MaybeUninit<RwLock<Record>>; NUM_BUCKETS] =
        //     unsafe { MaybeUninit::uninit().assume_init() };

        // // Initialize each element of the array safely.
        // for elem in &mut array[..] {
        //     *elem = MaybeUninit::new(RwLock::new(Record::empty()));
        // }

        // // SAFETY: All elements of the array are initialized, so this is now safe.
        // let initialized_array: [RwLock<Record>; NUM_BUCKETS] =
        //     unsafe { std::mem::transmute(array) };
        Self((0..NUM_BUCKETS).map(|_| Record::empty()).collect())
    }

    const fn num_buckets() -> usize {
        NUM_BUCKETS
    }

    #[inline(always)]
    fn process_at(&mut self, mut hashed_idx: usize, city_name: &'a [u8], value: i16) {
        // TODO: get unchecked
        loop {
            let bucket = &mut self.0[hashed_idx];

            match &bucket.name {
                Some(n) if *n == city_name => {
                    // println!("Idx {hashed_idx} contains {:?}... processing", std::str::from_utf8(city_name).unwrap());
                    break;
                }
                Some(_other) => {
                    // println!("Idx {hashed_idx} is filled with {:?}, cannot input {:?}", std::str::from_utf8(other).unwrap(),std::str::from_utf8(city_name).unwrap());
                    hashed_idx += 1;
                    hashed_idx %= NUM_BUCKETS
                }
                None => {
                    // println!("Idx {hashed_idx} is empty, inputting {:?}", std::str::from_utf8(city_name).unwrap());
                    *bucket = Record::new_with_initial(city_name, value);
                    return;
                }
            }
            self.0[hashed_idx].process(value);
        }
    }

    fn to_sorted(self) -> Vec<Record<'a>> {
        let mut r: Vec<_> = self.0.into_iter().filter(|r| r.name.is_some()).collect();
        // TODO: Check sort unstable for difference
        r.sort();
        r
    }

    fn merge(& mut self, mut other: Measurements<'a>) {
        for e in other.0.iter_mut().filter(|e|e.name.is_some()) {
            // Temporarily take out the name to avoid borrow issues
            self.find_and_merge( e) 
        }
    }

    fn find_and_merge<'b:'a>(& mut self,  other:&mut  Record<'b>) {
        let mut hash = INITIAL_HASH;
        // TODO: unchecked indexing
        // TODO: check from the back instead of the front of the string
        // Saves this while loop, but makes check more complicated
        // Might work for longer string names
        let name = & other.name.unwrap();
        if other.name.is_none() {
            return
        }

        for c in other.name.unwrap() {
            hash ^= *c as u64;
            hash = hash.wrapping_mul(HASH_WRAP_MUL);
        }

        let mut hashed_idx = (hash % Measurements::num_buckets() as u64) as usize;
        // set i before semi_colon
        loop {
            let bucket = &self.0[hashed_idx];

            match &bucket.name {
                Some(n) if n == name => {
                    // println!("Idx {hashed_idx} contains {:?}... processing", std::str::from_utf8(city_name).unwrap());
                    break;
                }
                Some(_other) => {
                    // println!("Idx {hashed_idx} is filled with {:?}, cannot input {:?}", std::str::from_utf8(other).unwrap(),std::str::from_utf8(city_name).unwrap());
                    hashed_idx += 1;
                    hashed_idx %= NUM_BUCKETS
                }
                None => {
                    // println!("Idx {hashed_idx} is empty, inputting {:?}", std::str::from_utf8(city_name).unwrap());
                    return;
                }
            }
        }
        self.0[hashed_idx].merge(other);
    }
}

/**
 * Returns shift necessary for next_starting_point
 */
fn fast_hash<'a>(s: &'a [u8], start: usize, measurements: &mut Measurements<'a>) -> usize {
    let (hashed_idx, name_end) = hash_find(s, start);

    // skip ';'
    let mut i = name_end + 2;

    let is_negative = s[i] == b'-';
    if is_negative {
        // skip '-', if it exists
        i += 1
    }

    let mut value = 0;
    // TODO: Check loop unrolled instead of *100 , *10
    if s[i + 1] == b'.' {
        // handle X.Y
        value = (s[i] - 48) as i16 * 10;
        i += 2;
        value += (s[i] - 48) as i16;
    } else if s[i + 2] == b'.' {
        // handle XY.Z
        value = (s[i] - 48) as i16 * 100;
        i += 1;
        value += (s[i] - 48) as i16 * 10;
        i += 2;
        value += (s[i] - 48) as i16;
    }

    if is_negative {
        value *= -1;
    }

    measurements.process_at(hashed_idx, &s[start..=name_end], value);
    // skip paragraph
    i + 2
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

fn improved_parsing(file_name: &str) -> Vec<u8> {
    const NUM_CORES: usize = 1;
    let source = std::fs::File::open(file_name).unwrap();
    let file_len = source.metadata().unwrap().len() as usize;
    let source = unsafe { MmapOptions::new().map(&source).unwrap() };

    let chunk_size = file_len / NUM_CORES;
    let mut chunk_borders = [(0, 0); NUM_CORES];
    chunk_borders
        .iter_mut()
        .enumerate()
        .for_each(|(i, (a, b))| {
            *a = i * chunk_size;
            *b = (i + 1) * chunk_size;
        });
    chunk_borders[NUM_CORES - 1].1 = file_len;
    println!("chunk_borders: {:?}", chunk_borders);
    thread::scope(|s| {
        // let sourceref: &[u8] = &source ;
        let mut handles = vec![];
        for chunk in chunk_borders.iter() {
            handles.push(s.spawn(|| {
                let mut measurements = Measurements::new();
                let mut start = chunk.0;
                let end = chunk.1;
                while start < end {
                    start = fast_hash(&source, start, &mut measurements);
                }
                measurements
            }));
        }
        let mut measurements: Measurements = handles.pop().unwrap().join().unwrap();

        // TODO: This is blocking, but shouldn't make a big difference
        for h in handles {
            let result = h.join().unwrap();
            measurements.merge(result);
        }

        // println!("{:?}", measurements);
        // 15 is an estimate of the averge size for the output of each city
        let mut result_buffer = Vec::with_capacity(NUM_BUCKETS * 15);
        result_buffer.push(b'{');

        // TODO: check pass by reference
        measurements
            .to_sorted()
            .into_iter()
            .for_each(|r| write_record(&mut result_buffer, r));
        // remove last ','
        result_buffer.pop();
        result_buffer.push(b'}');
        // println!("buffer len: {} ", result_buffer.len());
        result_buffer
        // println!("\nTook {:?} to process", timer.elapsed());
    })
}

fn write_record(
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

fn mean(sum: i64, count: i64) -> i16 {
    (sum / count) as i16
}

fn write_n(buffer: &mut Vec<u8>, value: i16) {
    if value < 0 {
        buffer.push(b'-')
    }

    let value = value.abs();

    if value >= 10 {
        buffer.push((value / 100) as u8 + b'0');
    }
    buffer.push(((value / 10) % 10) as u8 + b'0');
    buffer.push(b'.');
    buffer.push((value % 10) as u8 + b'0');
}

fn generate_results() {
    [10, 100, 10000, 1000000, 1000000000].iter().for_each(|n| {
        let builder = thread::Builder::new()
            .name("master_thread".to_string())
            .stack_size(size_of::<Measurements>() * 4); // Set the stack size to 4 MB

        let input_name = format!("../inputs/measurements_{n}.txt");
        let output_name = format!("../outputs/result_{n}.txt");

        let timer = Instant::now();
        let handle = builder.spawn(move || improved_parsing(&input_name));
        let contents = handle.unwrap().join().unwrap();
        std::fs::write(output_name, contents).unwrap();
        println!(
            "Took {} to parse {n} measurements",
            timer.elapsed().as_millis()
        );
    });
}

fn main() {
    // generate_results();
    const N: usize = 10;
    let builder = thread::Builder::new()
        .name("master_thread".to_string())
        .stack_size(size_of::<Measurements>() * 4); // Set the stack size to 4 MB

    let input_name = format!("../inputs/measurements_{N}.txt");
    let expected_name = format!("../outputs/result_{N}.txt");
    let expected = std::fs::read(expected_name).unwrap();
    let timer = Instant::now();
    let handle = builder.spawn(move || improved_parsing(&input_name));
    let result = handle.unwrap().join().unwrap();
    assert_eq!(std::str::from_utf8(&expected), std::str::from_utf8(result.as_slice()));
    println!(
        "Took {} to parse {N} measurements",
        timer.elapsed().as_millis()
    );
    // let source = std::fs::read("../inputs/measurements_3.txt").unwrap();
    // let end = source.len();
    // let mut start = 0;
    // let measurements = Measurements::new();
    // while start < end {
    //     let inc = fast_hash(&source, start, &measurements);
    //     println!("{inc}, {}", std::str::from_utf8(&source[start..start + inc -1]).unwrap());
    //     start += inc;
    // }

    // println!("File has been generated");
    // let a: AHashSet<&str> = source.lines().map(|l|l.split_once(';').unwrap().0).collect();
    // println!("Num cities {}", a.len());
}

trait DropAfter: num::Float {
    fn drop_decimals_after(self, decimal_places: u32) -> Self;
}

impl DropAfter for f32 {
    fn drop_decimals_after(self, decimal_places: u32) -> Self {
        let ten_power = 10_u32.pow(decimal_places) as f32;
        (self * ten_power).round() / ten_power
    }
}

#[cfg(test)]
mod tests {

    use crate::{improved_parsing, mean, record::Record, write_n, write_record, DropAfter};

    #[test]
    fn measurements_101() {
        let result = improved_parsing("../inputs/measurements_100.txt");
        let expected = std::fs::read("../outputs/results_100.txt").unwrap();
        assert_eq!(expected, result.as_slice())
    }

    #[test]
    fn write_n_test() {
        let mut buff = vec![];
        let inputs = [-999, 999, 0, 1, 10, 100, -1, -10, -100, -99, 99];

        for i in inputs {
            buff.clear();
            write_n(&mut buff, i);
            let result: f32 = match std::str::from_utf8(&buff) {
                Ok(result) => result.parse().unwrap(),
                Err(_) => {
                    todo!()
                }
            };
            //
            assert_eq!(result.drop_decimals_after(1), (i as f32) / 10.0);
        }
    }

    #[test]
    fn write_city_test() {
        let inputs = [(
            Record {
                name: Some("Porto".as_bytes()),
                max: 912,
                min: -881,
                count: 70,
                sum: 70 * 123,
            },
            format!(
                "Porto=-88.1/91.2/{},",
                ((mean(70 * 123, 70) as f32) / 10.0).drop_decimals_after(1)
            ),
        )];

        let mut buff = vec![];
        for (input, expected) in inputs {
            buff.clear();
            write_record(&mut buff, input);
            assert_eq!(expected, std::str::from_utf8(buff.as_slice()).unwrap());
        }
    }

    #[test]
    fn test_mean() {
        #[allow(clippy::identity_op)]
        let mean = mean(999 - 999 + 10 - 15, 4);
        assert_eq!(mean, -1)
    }
}
