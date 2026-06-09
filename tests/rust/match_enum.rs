enum Colour {
    Red(usize, usize),
    Green(usize),
    Blue,
}

fn test() -> usize {
    let g: Colour = Colour::Green(42);
    match g {
        Colour::Red(_, _) | Colour::Blue => 1,
        Colour::Green(n) => match Colour::Blue {
            Colour::Green(_) | Colour::Blue | Colour::Red(_, _) => match Colour::Red(5, 6) {
                Colour::Blue => 2,
                Colour::Green(_) | Colour::Red(_, _) => match Colour::Green(4) {
                    Colour::Red(_, _) | Colour::Green(_) => n,
                    Colour::Blue => 3,
                },
            },
        },
    }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
