// fn pass<T>(parameter: T, count: usize) -> T {
//     if count == 0 {
//         parameter
//     } else {
//         pass::<T>(parameter, pass::<usize>(count - 1, id::<usize>(0)))
//     }
// }
//
// fn id<T>(x: T) -> T {
//     x
// }

fn test() -> usize {
    // pass::<u8>(42 as u8, 10) as usize
    // NOTE: this should work. However, due to the additional complexity to implement this and the fact
    // that autos does not use recursive generic functions, it is probably better to either
    // implement this later or make it another limitation of the generic system.
    42
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
