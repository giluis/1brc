#[derive(Debug, Clone, Copy, Ord, Eq, PartialEq, PartialOrd)]
pub struct Record<'a> {
    // start, end
    pub name: Option<&'a [u8]>,
    pub min: i16,
    pub max: i16,
    pub sum: i64,
    pub count: i64,
}

impl<'a> std::fmt::Display for Record<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!(
            "{{name: {:?}, min: {}, max: {}, sum: {}, count: {}}}",
            self.name.map(|s| std::str::from_utf8(s).unwrap()),
            self.min,
            self.max,
            self.sum,
            self.count
        );
        write!(f, "{s}")
    }
}

impl<'a> Record<'a> {
    pub fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }

    // pub fn name<'a>(&'a self, source: &'a [u8]) -> &[u8] {
    //     if let Some((start, end)) = self.name {
    //         &source[start..end]
    //     } else {
    //         panic!("Tried to access name from record slot")
    //     }
    // }
    // TODO: check inline always
    // TODO: check value as (u8,u8) instead of u16
    pub fn process(&mut self, value: i16) {
        // TODO: unchecked operations
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value as i64;
        self.count += 1;
    }

    pub fn new_with_initial(name: &'a [u8], value: i16) -> Self {
        Self {
            name: Some(name),
            min: value,
            max: value,
            sum: value as i64,
            count: 1,
        }
    }

    pub fn empty() -> Self {
        Self {
            name: None,
            min: i16::MAX,
            max: i16::MIN,
            sum: 0,
            count: 0,
        }
    }

    pub fn merge(&mut self, other :& Self) {
        self.max = self.max.max(other.max);
        self.min = self.min.min(other.min);
        self.sum += other.sum;
        self.count += other.count;
    }
}
