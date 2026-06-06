// definition generates no code, only symbol table entry
enum Colour {
    Red(usize),  // discriminator 0; offset 0 (usize)
    Blue,        // discriminator 1; offset 8 (nothing)
    Green(char), // discriminator 2; offset 8
}
// total size := sizeof(usize) + max{sizeof(usize), sizeof(()), sizeof(char)}
//             = 8 + 8 = 16
// => [2 x i64]

fn main() {
    let colour = my_function(Colour::Green('a'));

    match &colour {
        Colour::Red(val) => println!("It's red"),
        // copy nothing
        Colour::Blue => println!("It's blue"),
        x => println!("It's green"),
    }

    let p = unsafe { malloc(size_of::<Colour>()) as *mut Colour };
    unsafe { *p = colour };

    // match and let code should be very similar, so that I can share code
    // something like expr always is ptr, but only if copy is needed (variable), then load & store
    // to copy
}

fn my_function(colour: Colour) -> Colour {
    match colour {
        Colour::Red(val) => return Colour::Red(42),
        x => x,
    };

    Colour::Green('c')
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
}
