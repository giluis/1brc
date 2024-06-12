#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd)]
pub struct Record<'a> {
    // start, end
    pub name: Option<&'a [u8]>,
    pub min: u16,
    pub max: u16,
    pub sum: u32,
    pub count: u32,
}

impl <'a> Record<'a> {
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
    pub fn process(&mut self, value: u16) {
        // TODO: unchecked operations
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value as u32;
        self.count += 1;
    }

    pub fn new_with_initial(name: &'a [u8], value: u16) -> Self {
        Self {
            name: Some(name),
            min: value,
            max: value,
            sum: value as u32,
            count: 1,
        }
    }

    pub fn empty() -> Self{
        Self {
            name: None,
            min: u16::MAX,
            max: u16::MIN,
            sum: 0 as u32,
            count:0,
        }

    }

}
