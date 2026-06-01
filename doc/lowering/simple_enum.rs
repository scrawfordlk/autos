enum Animal {
    Dog,
    Cat(usize),
    Seal(u8),
}

fn main() {
    let animal: Animal = Animal::Cat(10);

    std::process::exit(match animal {
        Animal::Dog => 1,
        Animal::Cat(value) => value + 32,
        Animal::Seal(value) => value as usize,
    })
}
