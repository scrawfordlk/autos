enum Colour {
    Red,
    Green,
    Blue,
}

fn test() -> usize {
    let g: Colour = Colour::Green;
    match g {
        Colour::Red | Colour::Blue => 1,
        Colour::Green => match Colour::Blue {
            Colour::Green | Colour::Blue | Colour::Red => match Colour::Red {
                Colour::Blue => 2,
                Colour::Green | Colour::Red => match Colour::Green {
                    Colour::Red | Colour::Green => 42,
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
