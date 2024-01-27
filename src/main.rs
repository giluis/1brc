use baseline::baseline;
use record::Record;
use std::io::prelude::Write;
use std::io::stdout;

mod baseline;
#[allow(dead_code)]
mod generate;
mod record;

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
    let idx = (hash % 10_000) as usize;
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
    // TODO: check transmute [0u64;20_000] here (Option<&str> takes as much space as &str)
    let mut measurements = [(None, Record::empty()); 10_000];
    // let mut hasher = RandomState::new().build_hasher();
    while start < source.len() {
        start += fast_hash(&source, start, &mut measurements);
    }
    let mut buf = Vec::with_capacity(10_000 * (14 + 15));
    measurements.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    measurements.into_iter().for_each(|(city_name, record)| {
        if let Some(name) = city_name {
            // dbg!(String::from_utf8(name.into()));
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
    // let a = check(10_000, improved_parsing);
    // improved_parsing(1_000_000_000);
    // generate_file(100);
    // let mut hasher = AHasher::default();

    improved_parsing(100);
    println!("\n");
    baseline(100)

    // let NUM_AVG = 10;

    // let avg = (0..NUM_AVG).map(|_| {
    //     let timer = Instant::now();
    //     let mut n:i32 = 0;
    //     for i in 0..100_000{
    //         n += i * 2;
    //         if n > 50{
    //             n /= 100;
    //         }
    //         n += 10 + i;
    //         n *= 3;
    //     }
    //     println!("result: {n}");
    //     timer.elapsed().as_micros()
    // }).sum::<u128>() / NUM_AVG;

    // println!("took {}\n\n", avg);
    // let avg = (0..NUM_AVG).map(|_| {
    //     let timer = Instant::now();
    //     let mut n:u32 = 0;
    //     for i in 0..100_000{
    //         n += i * 2;
    //         if n > 50{
    //             n /= 100;
    //         }
    //         n += 10 + i;
    //         n *= 3;
    //     }
    //     println!("result: {n}");
    //     timer.elapsed().as_micros()
    // }).sum::<u128>() / NUM_AVG;
    // println!("took {}\n\n", avg);
    // println!("took {}\n\n", avg);
    // println!("{}", (0 + 999 + 1998) / 30);
    // println!("{}", (0 + 999 + 1998) / 3 % 10);
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
