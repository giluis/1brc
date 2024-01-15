use self::stackvec::StackVec;

mod stackvec;

enum Jump {
    Node(usize),
    End(usize),
}

struct Rules {
    starting: StackVec<(char, Jump), 10_000>,
    rules: StackVec<(char, Jump), 20_000>,
}

impl Rules {
    fn insert(&mut self, input: &str) {
        

    }

}
