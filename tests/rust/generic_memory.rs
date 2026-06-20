fn test() -> usize {
    let mut x: usize = f::<usize>(42);
    let c: char = f::<char>('a');
    let mut y: usize = f::<usize>(33);
    swap::<usize>(&mut x, &mut y);
    (x == 33) as usize + (y == 42) as usize + 40
}

fn f<T>(value: T) -> T {
    g::<T>(value)
}

fn g<T>(value: T) -> T {
    value
}

fn swap<T>(a: &mut T, b: &mut T) {
    unsafe {
        let temp: *mut T = alloc_one::<T>();
        let a: *mut T = a as *mut T;
        let b: *mut T = b as *mut T;
        memcpy_one::<T>(temp, a);
        memcpy_one::<T>(a, b);
        memcpy_one::<T>(b, temp);
    }
}

unsafe fn memcpy_one<T>(dest: *mut T, src: *mut T) {
    let byte_count: usize = size_of::<T>();
    let p: *mut u8 = dest as *mut u8;
    let q: *mut u8 = src as *mut u8;
    let mut i: usize = 0;
    while i < byte_count {
        unsafe { *ptr_add::<u8>(p, i) = *ptr_add::<u8>(q, i) };
        i = i + 1;
    }
}

unsafe fn alloc_one<T>() -> *mut T {
    unsafe { malloc(size_of::<T>()) as *mut T }
}

fn ptr_add<T>(p: *mut T, n: usize) -> *mut T {
    (p as usize + n * size_of::<T>()) as *mut T
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
    fn malloc(size: usize) -> *mut u8;
}
