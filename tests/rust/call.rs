fn main() -> usize {
    pow(2, 5) + pow(3, 2) + pow(9, 0)
}

fn pow(base: usize, exp: usize) -> usize {
    while exp == 0 {
        return 1;
    }

    return base * pow(base, exp - 1);
}
