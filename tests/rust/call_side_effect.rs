fn main() -> usize {
    let mut x: usize = 1;
    mutate(&mut x);
    x
}

fn mutate(reference: &mut usize) {
    *reference = 42;
}
