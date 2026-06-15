// fn other<T>(param: &T) -> &T {
//    f::<usize>(&42, 10 as u8);
//    f::<u8>(&(42 as u8), 10 as u8);
//    param
// }
//
// fn f<T>(param: &T, count: u8) {
//    let mut i: usize = count as u8;
//    let value: &T = param;
//    while i > 0 {
//       i = i - 1;
//    }
// }

fn pass<T>(parameter: T) -> T {
    // let reference: &T = other::<T>(&parameter);
    parameter
}

fn test() -> usize {
    pass::<u8>(42 as u8); // type mismatch would say "T", not the instance
    42
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
