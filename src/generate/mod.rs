mod cities;
use std::{fs::File, io::prelude::Write, thread};
pub use cities::CITIES;
use rand::Rng;

pub fn generate_file(n:usize) {
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
                let value = if rng.gen::<bool>() {
                    &value[..]
                } else {
                    &value[1..]
                };
                buf += std::str::from_utf8(value).unwrap();
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

fn generate_city<'a>(rng: &mut impl Rng) -> (&'a str, [u8; 5]) {
    let name = get_city();
    let whole_part = rng.gen_range(0u8..99u8);
    let decimal_part = rng.gen_range(0u8..=9u8);
    (
        name,
        [
            45,
            if whole_part >= 9 {
                0 + 48
            } else {
                whole_part / 10 + 48
            },
            whole_part % 10 + 48,
            46,
            decimal_part + 48,
        ],
    )
}

fn get_city() -> &'static str {
    let idx = rand::thread_rng().gen_range(0..cities::CITIES.len());
    CITIES[idx]
}
