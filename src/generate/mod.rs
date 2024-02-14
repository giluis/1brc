mod cities;
pub use cities::CITIES;
use rand::{prelude::SliceRandom, Rng};
use std::{fs::File, io::prelude::Write, thread};

lazy_static::lazy_static! {
    static ref CITY_IDXS: [usize; 10_000] = gen_city_idxs();
}

fn gen_city_idxs() -> [usize; 10_000] {
    let mut rng = rand::thread_rng();
    [(); 10_000].map(|_| rng.gen_range(0..CITIES.len()))
}

pub fn generate_file(n: usize) {
    let num_threads = 6;
    let mut batch_size = n / num_threads;
    let mut handles = vec![];
    let mut prev = 0;
    for t in 0..num_threads {
        if t == num_threads - 1 {
            batch_size = n - t * batch_size;
        }

        handles.push(thread::spawn(move || {
            let mut thread_buf = Vec::with_capacity(15 * n / num_threads);
            let mut rng = rand::thread_rng();
            (prev..prev + batch_size).for_each(|_| {
                add_measurement(&mut thread_buf, &mut rng);
            });
            thread_buf
        }));
        prev += batch_size;
    }
    let mut result = Vec::<u8>::with_capacity(15 * n);
    for t in handles {
        result.extend(t.join().unwrap());
    }

    File::create(format!("measurements_{n}.txt"))
        .unwrap()
        .write_all(&result)
        .unwrap();
}

fn add_measurement(buff: & mut Vec<u8> , rng: &mut impl Rng) {
    buff.extend(get_city().as_bytes());
    buff.push(b';');
    let is_negative = rng.gen::<bool>();
    if is_negative {
        buff.push(b'-');
    }
    let whole_part = rng.gen_range(0u8..99u8);
    let decimal_part = rng.gen_range(0u8..=9u8);
    if whole_part > 9 {
        buff.push(48 + whole_part / 10);
    }
    buff.push(48 + whole_part % 10);
    buff.push(b'.');
    buff.push(48 + decimal_part);
    buff.push(b'\n');
}

fn get_city() -> &'static str {
    let idx = CITY_IDXS.choose(&mut rand::thread_rng()).unwrap();
    CITIES[*idx]
}
