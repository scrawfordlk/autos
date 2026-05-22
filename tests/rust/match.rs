fn main() -> usize {
    let n: u8 = 4 as u8;

    match n {
        10 => return 1,
        4 => match false {
            true => 2,
            false => match 'a' {
                'b' => 4,
                'a' => return 42,
                _ => 5,
            },
        },
        100 => return 6,
    }
}
