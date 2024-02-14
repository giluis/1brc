
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd)]
pub struct Record<'a> {
    pub name: &'a [u8],
    pub min: u16,
    pub max: u16,
    pub sum: u32,
    pub count: u32,
}

impl <'a> std::cmp::Ord for Record<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(other.name)
    }
}

impl <'a> Record<'a> {
    // TODO: check inline always
    // TODO: check value as (u8,u8) instead of u16
    pub fn process(&mut self, value: u16) {
        // TODO: unchecked operations
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value as u32;
        self.count += 1;
    }

    pub fn new_with_initial(name: &'a [u8], value: u16) -> Self {
        Self {
            name,
            min: value,
            max: value,
            sum: value as u32,
            count: 1
        }
    }
}