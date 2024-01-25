mod cities;
pub use cities::CITIES;
use rand::{prelude::SliceRandom, Rng};
use std::{fs::File, io::prelude::Write, thread};

lazy_static::lazy_static! {
    static ref CITY_IDXS: [usize; 10] = gen();
}

fn gen() -> [usize; 10] {
    let mut rng = rand::thread_rng();
    [(); 10].map(|_| rng.gen_range(0..CITIES.len()))
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
            let mut buf = String::with_capacity(15 * n / num_threads);
            (prev..prev + batch_size).for_each(|_| {
                let mut rng = rand::thread_rng();
                let (name, value) = generate_city(&mut rng);
                buf += name;
                buf.push(';');
                buf += std::str::from_utf8(&value).unwrap();
                buf.push('\n');
            });
            buf
        }));
        prev += batch_size;
    }
    let mut buf = String::with_capacity(15 * n);

    for t in handles {
        buf += &t.join().unwrap()
    }

    File::create(format!("measurements_{n}.txt"))
        .unwrap()
        .write_all(buf.as_bytes())
        .unwrap();
}

fn generate_city(rng: &mut impl Rng) -> (&str, Vec<u8>) {
    let name = get_city();
    let whole_part = rng.gen_range(0u8..99u8);
    let decimal_part = rng.gen_range(0u8..=9u8);
    let mut value = vec![];
    let is_negative = rng.gen::<bool>();
    if is_negative {
        value.push(b'-');
    }
    if whole_part > 9 {
        value.push(48 + whole_part / 10);
    }
    value.push(48 + whole_part % 10);
    value.push(b'.');
    value.push(48 + decimal_part);
    (name, value)
}

fn get_city() -> &'static str {
    let idx = CITY_IDXS.choose(&mut rand::thread_rng()).unwrap();
    CITIES[*idx]
}
