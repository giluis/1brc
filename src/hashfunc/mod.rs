#[deprecated]



use ahash::AHashMap;

use self::stackvec::StackVec;

mod stackvec;

enum Jump {
    Node(char, usize, Option<usize>),
    End(usize),
    NotSet,
}

// impl<'a> Jump<'a> {
//     fn next(&self, c: &char) -> (bool, usize) {
//         match self {
//             Jump::Node(t, if_true, _) if t == c => (false, *if_true),
//             Jump::Node(t, _, if_false) if t != c => (false, *if_false),
//             Jump::Node(_, _, _) => {

//                 unreachable!("Conditions in other JUmp::Node arms are exhaustive")
//             }
//             Jump::End(_, _) => todo!(),
//             _ => todo!()
//         }
//     }
// }

struct FaStMAp {
    starting: AHashMap<char, usize>,
    rules: StackVec<Jump, 20_000>,
    data: StackVec<f32, 10_000>,
}

impl FaStMAp {
    fn get_or_insert(&mut self, input: &str) -> usize {
        let mut curr_idx = match self.starting.get(&input.chars().next().unwrap()) {
            Some(idx) => *idx,
            None => return self.insert(None, input)
        };
        let mut should_insert = false;
        for (i, c) in input.chars().enumerate() {
            match &mut self.rules[curr_idx] {
                Jump::Node(target, if_true, _) if c == *target => {
                    curr_idx = *if_true;
                }
                Jump::Node(target, _, Some(if_false)) if c != *target => {
                    curr_idx = *if_false;
                }
                Jump::Node(target, _, None) if c != *target => {
                    should_insert = true;
                }
                Jump::End(idx) => return *idx,
                _ => unreachable!("This should never happen, since all possible cases are covered in this match statement"),
            }
            if should_insert {
                return self.insert(Some(curr_idx), &input[i..]);
            }
        }
        curr_idx
    }

    fn insert(&mut self, prev_idx: Option<usize>, i: &str) -> usize {
        match prev_idx {

            Some(p) => {
                match &mut self.rules[p] {
                    Jump::Node(_, _, ref mut if_false ) => unimplemented!(),
                    Jump::End(_) => unimplemented!(),
                    Jump::NotSet => unimplemented!(),
                }

            },
            None => todo!(),
        }
    }
}
