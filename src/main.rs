mod cities;

struct Analysis {
    min: f32,
    max: f32,
    sum: f32,
    count: usize,
}

impl std::fmt::Debug for Analysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { min, max, .. } = self;
        write!(f, "(min: {min}, max: {max}, mean: {:.1})", self.mean())
    }
}

impl Analysis {
    fn mean(&self) -> f32 {
        self.sum / self.count as f32
    }

    fn new() -> Self {
        Self {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
            sum: 0.0,
            count: 0,
        }
    }

    fn process(&mut self, value: f32) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.count += 1;
    }
}

fn analyze() {
    let file = std::fs::File::open("./cities.txt").unwrap();
    let bufread = BufReader::new(file);
    let mut map = HashMap::<String, Analysis>::new();
    let timer = Instant::now();

    for line in bufread.lines() {
        let line = line.unwrap();
        let (city, value) = line.split_once(';').unwrap();
        map.entry(city.to_owned())
            .or_insert(Analysis::new())
            .process(value.parse().unwrap());
    }

    let elapsed = timer.elapsed();
    println!("{:?}", map);
    println!("Took {} ms", elapsed.as_millis());
}

use cities::CITIES;
use unic_ucd_category::GeneralCategory;
use unicode_segmentation::UnicodeSegmentation;

fn contains_invisible_chars(s: &str) -> bool {
    s.graphemes(true)
        .flat_map(|g| g.chars())
        .any(|c| match GeneralCategory::of(c) {
            GeneralCategory::Control
            | GeneralCategory::Format
            | GeneralCategory::Unassigned
            | GeneralCategory::PrivateUse
            | GeneralCategory::Surrogate
            | GeneralCategory::SpaceSeparator => true,
            _ => false,
        })
}

fn main() {
    // (0..100).for_each(|_|  {
    //     let mut rng = rand::thread_rng();
    //     let (name, value )= generate_city(&city_names, &mut rng);
    //     println!("{name};{}", from_utf8(if rng.gen::<bool>() {
    //         &value[..]
    //     } else {
    //         &value[1..]
    //     }).unwrap());
    // });
    let timer = Instant::now();
    generate_file();
    let elapsed = timer.elapsed().as_secs();
    println!("{:?}", elapsed);
    // let city_names = std::fs::read_to_string("./city_names.csv").unwrap();
    // let contents = city_names
    //     .lines()
    //     .map(|s| s.split_once(',').unwrap().0)
    //     .filter(|s| !contains_invisible_chars(s))
    //     .fold("\"".to_owned(), |a, b| a + "\"" + "," + "\n" + "\"" + b);
    // std::fs::write("new_city_names.rs", contents).unwrap();
}

/* Generation */
use rand::prelude::{random, Rng};
use std::{
    cmp::min,
    collections::HashMap,
    fs::{remove_file, File},
    io::{
        prelude::{BufRead, Write},
        BufReader,
    },
    str::from_utf8,
    time::Instant,
};

fn generate_city<'a>(rng: &mut impl Rng) -> (&'a str, [u8; 5]) {
    let name = get_city();
    let value = random::<f32>() * 99.0 * 2.0 - 99.0;
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

fn get_city<'a>() -> &'static str {
    let idx = rand::thread_rng().gen_range(0..cities::CITIES.len());
    CITIES[idx]
}

fn generate_file() {
    let _ = std::fs::remove_file("./cities.txt");
    let mut buf = String::with_capacity(12_000_000);
    (0..100_000_000).for_each(|_| {
        let mut rng = rand::thread_rng();
        let (name, value) = generate_city(&mut rng);
        buf += name;
        buf.push(',');
        let value = if rng.gen::<bool>() {
            &value[..]
        } else {
            &value[1..]
        };
        buf += std::str::from_utf8(value).unwrap();
        buf.push('\n');
    });
    File::create("./cities.txt")
        .unwrap()
        .write_all(buf.as_bytes())
        .unwrap();
}
