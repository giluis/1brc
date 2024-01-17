use std::ops::{Index, IndexMut};
pub struct StackVec<T, const N: usize> {
    data: [T; N],
    len: usize,
}


impl<T, const N: usize> Index<usize> for StackVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { self.data.get_unchecked(index) }
    }
}

impl<T, const N: usize> IndexMut<usize> for StackVec<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut <Self as Index<usize>>::Output {
        unsafe { self.data.get_unchecked_mut(index) }
    }
}

impl<T, const N: usize> StackVec<T, N> {
    pub fn push(&mut self, elem: T) {
        assert!(self.len < N - 1);
        self.push_unchecked(elem)
    }

    pub fn push_unchecked(&mut self, elem: T) {
        self.data[self.len - 1] = elem;
        self.len += 1;
    }

    pub const fn cap(&self) -> usize {
        N
    }
}

