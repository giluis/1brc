#![feature(ascii_char)]
#![allow(clippy::type_complexity)]
#![feature(let_chains)]
#![feature(maybe_uninit_uninit_array)]
#![feature(array_windows)]
#![feature(generic_const_exprs)]
#![feature(generic_arg_infer)]

use itertools::Itertools;
use memmap2::MmapOptions;
use rayon::iter::Chain;
use record::Record;
use std::{
    fs,
    io::{stdout, Write},
    mem::MaybeUninit,
    ptr::hash,
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

const NUM_BUCKETS: usize = 15;

#[derive(Clone)]
struct Measurements<'a>(Vec<Record<'a>>);

struct MeasurementsIterator {
    idx: usize,
}

impl<'a> std::fmt::Debug for Measurements<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = "{".to_owned();
        for r in self.0.iter().filter(|r| r.count != 0) {
            result += &r.to_string();
        }
        result += "}";
        write!(f, "{result}")
    }
}

impl<'a> Measurements<'a> {
    fn new() -> Self {
        // TODO: try with stack allocated array
        // tried before, but stack size wasn't sufficient
        Self((0..NUM_BUCKETS).map(|_| Record::empty()).collect())
    }

    const fn num_buckets() -> usize {
        NUM_BUCKETS
    }

    // #[inline(always)]
    fn process_at(&mut self, hashed_idx: usize, name: &'a [u8], value: i16) {
        // TODO: avoid name assignment all the time 
        self.0[hashed_idx].merge(&Record::new_with_initial(name, value));
        self.0[hashed_idx].name = name;
    }

    fn sorted(self) -> Vec<Record<'a>> {
        let mut r: Vec<_> = self.0.into_iter().filter(Record::is_empty).collect();
        // TODO: Check sort unstable for difference
        r.sort();
        r
    }

    fn merge(&mut self, other: Measurements<'a>) {
        for e in other.0.iter().filter(|r| !r.is_empty()) {
            self.merge_one(e)
        }
    }

    fn hash(&mut self, record: &Record) -> usize {
        let mut hash = INITIAL_HASH;
        assert!(!record.is_empty(), "Cannot hash an empty record");
        let mut hashed_idx = {
            for c in record.name {
                hash ^= *c as u64;
                hash = hash.wrapping_mul(HASH_WRAP_MUL);
            }
            (hash % Measurements::num_buckets() as u64) as usize
        };

        // set i before semi_colon
        loop {
            let found = &self.0[hashed_idx];
            if found.is_empty() || found.name == record.name {
                break;
            } else {
                hashed_idx += 1;
                hashed_idx %= NUM_BUCKETS;
            }
        }
        hashed_idx
    }

    fn merge_one<'b: 'a>(&mut self, other: &Record<'b>) {
        let hashed_idx = self.hash(other);
        self.0[hashed_idx].merge(other)
    }

    // TODO: unchecked indexing
    // TODO: #[inline(always)]
    fn hash_until_char(&self, s: &[u8], start: usize, stop_char: u8) -> (usize, usize) {
        let mut city_name_end = start;

        let mut hash = INITIAL_HASH;
        // TODO: double access, perhaps caching would improve efficiency
        let mut hashed_idx = {
            while s[city_name_end] != stop_char{
                hash ^= s[city_name_end] as u64;
                hash = hash.wrapping_mul(HASH_WRAP_MUL);
                city_name_end += 1;
            }
            (hash % Measurements::num_buckets() as u64) as usize
        };

        loop {
            let found = &self.0[hashed_idx];
            if found.is_empty() || found.name == &s[start..city_name_end]{
                break;
            } else {
                hashed_idx += 1;
                hashed_idx %= NUM_BUCKETS;
            }
        }
        // set city_name_end to before semi_colon
        (hashed_idx, city_name_end)
    }
}

/**
 * Returns next starting point after this city and its value have been parsed
 */
fn fast_hash<'a>(s: &'a [u8], start: usize, measurements: &mut Measurements<'a>) -> usize {
    // TODO: avoid double measurements access in hash_until_char and process_at
    let (hashed_idx, city_name_end) = measurements.hash_until_char(s, start, b';');
    let mut i = city_name_end + 1;

    // skip '-', if it exists
    let is_negative = s[i] == b'-';
    if is_negative {
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

    measurements.process_at(hashed_idx, &s[start..city_name_end] ,value);
    // skip paragraph
    i + 2
}

trait PreAppend: Iterator + Sized {
    fn prepend(self, item: Self::Item) -> std::iter::Chain<std::iter::Once<Self::Item>, Self>;
    fn append(self, item: Self::Item) -> std::iter::Chain<Self, std::iter::Once<Self::Item>>;
}

impl <T: Iterator> PreAppend for T {
    fn prepend(self, item: Self::Item) -> std::iter::Chain<std::iter::Once<Self::Item>, Self> {
        std::iter::once(item).chain(self)
    }
    fn append(self, item: Self::Item) -> std::iter::Chain< Self, std::iter::Once<Self::Item>> {
        self.chain(std::iter::once(item))
    }
}

const AVG_CITY_NAME_LEN: usize = 15;

fn improved_parsing(file_name: &str) -> Vec<u8> {
    const NUM_CORES: usize = 4;

    let source = std::fs::File::open(file_name).unwrap();
    let file_len = source.metadata().unwrap().len() as usize;
    let source = unsafe { MmapOptions::new().map(&source).unwrap() };

    assert!(source.len() >  NUM_CORES * (AVG_CITY_NAME_LEN + 5));
    let chunk_size = file_len / NUM_CORES;
    let chunk_borders: Vec<(_, _)> = (1..NUM_CORES)
        .map(|i| i * chunk_size)
        .map(|mut p| {
            while source[p] != b'\n' {
                p += 1
            }
            p + 1
        })
        .append(file_len)
        .prepend(0)
        .tuple_windows()
        .collect();
    dbg!("Here");

    thread::scope(|s| {
        // let sourceref: &[u8] = &source ;
        let mut handles = vec![];
        for  (i , chunk) in chunk_borders.iter().enumerate() {
            handles.push(s.spawn(|| {
                let mut measurements = Measurements::new();
                let mut start = chunk.0;
                let end = chunk.1;
                while start < end {
                    dbg!("In thread {}, start = {}", i, start);
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

        dbg!(&measurements);
        // println!("{:?}", measurements);
        // 15 is an estimate of the averge size for the output of each city
        let mut result_buffer = Vec::with_capacity(NUM_BUCKETS * 15);
        result_buffer.push(b'{');

        // TODO: check pass by reference
        measurements
            .sorted()
            .into_iter()
            .filter(|r|!r.is_empty())
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
    buff.extend_from_slice(record.name);
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
    const N: usize = 100;
    let builder = thread::Builder::new()
        .name("master_thread".to_string())
        .stack_size(size_of::<Measurements>() * 4); // Set the stack size to 4 MB

    let input_name = format!("../inputs/measurements_{N}.txt");
    let expected_name = format!("../outputs/result_{N}.txt");
    let expected = std::fs::read(expected_name).unwrap();
    let timer = Instant::now();
    let result = builder
        .spawn(move || improved_parsing(&input_name))
        .unwrap()
        .join()
        .unwrap();
    std::fs::write("temporary_result.txt", result).unwrap();
    // assert_eq!(
    //     std::str::from_utf8(&expected),
    //     std::str::from_utf8(result.as_slice())
    // );
    // println!("Took {:?} to parse {N} measurements", timer.elapsed());
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

    use std::io::BufRead;

    use crate::{
        fast_hash, improved_parsing, mean, record::Record, write_n, write_record, DropAfter,
        Measurements,
    };

    #[test]
    fn test_measurements_10() {
        let input = "Karauli;-95.6
Caucaguita;-74.7
Medina;-96.9
Owosso;-89.3
NorrkÃ¶ping;27.4
Rouyn-Noranda;-77.3
Karauli;6.8
Jincheng;0.7
Karachayevsk;-36.3
Miryang;-8.9";

        let expected = Measurements::new();
        let mut result = Measurements::new();
        let mut start = 0;
        while start < input.len() {
            start = fast_hash(input.as_bytes(), start, &mut result);
        }
        let expected_karauli = Record {
            name: "Karauli".as_bytes(),
            min: -956,
            max: 68,
            sum: -956 + 68,
            count: 2
        };
        assert_eq!(expected_karauli , *result.0.iter().find(|r|r.name == "Karauli".as_bytes()).unwrap());
        result.0.iter().for_each(|r|println!("{:?}", r));
    }

    #[test]
    fn test_fast_hash() {
        let cities = [
            ("city1", 1.2),
            ("mycity2", -8.2),
            ("ourcity1", 11.7),
            ("ourcity3", -89.2),
            ("city1", -1.2),
        ];

        let s = cities
            .iter()
            .fold(String::new(), |acc, (city_name, value)| {
                acc + &format!("{city_name};{value}") + "\n"
            });
        let source = s.as_bytes();

        let mut start = 0;
        let mut m = Measurements::new();

        for (city_idx, curr_city) in cities.into_iter().take(cities.len() - 1).enumerate() {
            start = fast_hash(source, start, &mut m);
            assert_eq!(
                start,
                source
                    .lines()
                    .take(city_idx + 1)
                    .map(|l| l.unwrap().len())
                    .sum::<usize>()
                    + city_idx
                    + 1
            );
            assert_eq!(
                &Record::init_from_tuple(curr_city),
                m.0.iter()
                    .find(|r| r.name == curr_city.0.as_bytes())
                    .unwrap()
            );
        }

        let start = fast_hash(source, start, &mut m);
        assert_eq!(start, source.len());
        let mut repeated_record = Record::init_from_tuple((cities[0].0, cities[0].1));
        repeated_record.min = (cities[cities.len() - 1].1 * 10.0).floor() as i16;
        repeated_record.sum += (cities[cities.len() - 1].1 * 10.0).floor() as i64;
        repeated_record.count += 1;
        // dbg!(m);

        assert_eq!(
            repeated_record,
            *m.0.iter()
                .find(|r| r.name == cities[0].0.as_bytes())
                .unwrap()
        );
    }

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
                name: "Porto".as_bytes(),
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
