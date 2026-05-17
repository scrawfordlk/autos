fn main() -> usize {
    let mut x: usize = 1;
    mutate(&mut x);
    42
}

fn mutate(reference: &mut usize) {
    *reference = 42;
}
