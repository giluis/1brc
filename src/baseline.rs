use std::{collections::HashMap, io::{stdout, Write}};

use itertools::Itertools;

use crate::{record::Record, Measurements};

trait ToByteBuffer {
    fn write_bytes(self, buffer: &mut Vec<u8>);
}

impl ToByteBuffer for i16 {
    fn write_bytes(self,buffer : &mut Vec<u8>) {
        if self < 0 {
            buffer.push(b'-')
        }

        if self.abs() >= 10 {
            let units = self% 10;
        }
        buffer.push((self/10) as u8);
        buffer.push((self%10) as u8);

        buffer.push(self % 10)
    }
}


fn results_to_string<'a, I: IntoIterator<Item=Record<'a>>>(map: I) -> Vec<u8>{
    	let mut result = vec![b'{'];
        for Record{name, min, max, sum, count} in map {
            result.extend(name.unwrap());
            result.push(b'=');
            result.push(min);
            result.push(max);
            result.push(sum)

        }
        result
}

fn baseline(data_points: &[u8]) -> String {
    let mut hashmap = HashMap::<&[u8], (f32, f32, f32, usize)>::new();
    let mut i = 0;
    let mut num_start = 0;
    let mut curr_city_start = 0;
    while i < data_points.len() {
        match &data_points[i] {
            b';' => {
                num_start = i + 1;
            }
            b'\n' => {
                // println!("{}", std::str::from_utf8(&data_points[num_start..i]).unwrap());
                let value = std::str::from_utf8(&data_points[num_start..i])
                    .unwrap()
                    .parse()
                    .unwrap();
                let entry = hashmap
                    .entry(&data_points[curr_city_start..num_start - 1])
                    .or_insert((0.0, 0.0, 0.0, 0));
                entry.0 = entry.0.min(value);
                entry.1 = entry.1.max(value);
                entry.2 += value;
                entry.3 += 1;
                curr_city_start = i + 1;
            },
            _ => {}
        }
        i += 1;
    }
    let mut result = hashmap.iter().collect::<Vec<_>>();
    result.sort_by(|(city1, _), (city2, _)| city1.cmp(city2));
    let mut result_str =
        result
            .iter()
            .fold("{".to_owned(), |acc, (city, (min, max, sum, count))| {
                acc + &format!(
                    "{}={:.1}/{:.1}/{:.1}, ",
                    std::str::from_utf8(city).unwrap(),
                    min,
                    max,
                    sum / (*count as f32)
                )
            });
    result_str.pop();
    result_str.pop();
    result_str += "}";
    result_str
}


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