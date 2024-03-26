#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd)]
pub struct Record {
    // start, end
    pub name: (usize, usize),
    pub min: u16,
    pub max: u16,
    pub sum: u32,
    pub count: u32,
}

impl Record {
    pub fn cmp(&self, other: &Self, source: &[u8]) -> std::cmp::Ordering {
        self.name(source).cmp(other.name(source))
    }

    pub fn name<'a>(&'a self, source: &'a [u8]) -> &[u8] {
        let (start, end) = self.name;
        &source[start..end]
    }
    // TODO: check inline always
    // TODO: check value as (u8,u8) instead of u16
    pub fn process(&mut self, value: u16) {
        // TODO: unchecked operations
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value as u32;
        self.count += 1;
    }

    pub fn new_with_initial(name: (usize, usize), value: u16) -> Self {
        Self {
            name,
            min: value,
            max: value,
            sum: value as u32,
            count: 1,
        }
    }
}
