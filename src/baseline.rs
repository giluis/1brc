use std::io::{stdout, Write};

use itertools::Itertools;


pub fn baseline(size: usize) {
    let a = std::fs::read_to_string(format!("../measurements_{size}.txt")).unwrap();
    let mut measurements = std::collections::HashMap::new();
    for l in a.lines() {
        let (city_name, value) = l.split_once(';').unwrap();
        let a: f64 = value.parse().unwrap();
        let entry = measurements.entry(city_name).or_insert((0.0, 0.0, 0.0, 0));
        entry.0 = a.min(entry.0);
        entry.1 = a.max(entry.1);
        entry.2 += a;
        entry.3 += 1;
    }
    let measurements: Vec<_> = measurements
        .iter()
        .sorted_by(|(a, _), (b, _)| a.cmp(b))
        .collect();

    let mut s = "{".to_owned();
    measurements
        .iter()
        .for_each(|(city_name, (min, max, sum, count))| {
            s += &format!(
                "{city_name}={:.1}/{:.1}/{:.1},",
                min,
                max,
                (10.0 * (sum / *count as f64)).ceil() / 10.0
            );
        });
    s.remove(s.len() - 1);
    s.push('}');
    stdout().lock().write_all(s.as_bytes()).unwrap();
}