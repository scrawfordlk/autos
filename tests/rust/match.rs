fn main() -> usize {
    let x: bool = true;
    let c: char = 'a';

    match 4 {
        10 => return 1,
        4 => match x {
            true => match c {
                'b' => 4,
                'a' => return 42,
                _ => 5,
            },
            false => 2,
        },
        100 => return 0,
    }
}
