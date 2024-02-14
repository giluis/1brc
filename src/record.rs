
#[derive(Debug, Clone, Copy)]
pub struct Record {
    pub min: u16,
    pub max: u16,
    pub sum: u32,
    pub count: u32,
}

impl Record {
    pub fn empty() -> Self {
        Record {
            min: u16::MAX,
            max: u16::MIN,
            sum: 0,
            count: 0,
        }
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
}