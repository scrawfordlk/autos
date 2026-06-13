enum Shape {
    Circle(char, u8, usize),
    Rect(Coords, Coords),
}

fn main() {
    let mut shape: Shape = Shape::Circle('a', 5 as u8, 42);
    shape = pass(Shape::Rect(Coords::Triple(5, 4, 3), Coords::Pair(7, 9)));

    shape = shape;
    let shape: Shape = shape;

    unsafe {
        let p: *mut Shape = malloc(8 + 4 * 8 * 2) as *mut Shape;
        *p = shape;
        &*&*p;
    };

    unsafe { exit(42) }
}

enum Coords {
    Unit,
    Single(usize),
    Pair(usize, usize),
    Triple(usize, usize, usize),
}

fn pass(shape: Shape) -> Shape {
    return shape;
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn exit(code: usize) -> !;
}
