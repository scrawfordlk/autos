enum Entry {
    Entry(usize, usize),
}

fn id<T>(e: T) -> T {
    e
}

fn allocate<T>(value: T) -> *mut T {
    let p: *mut T = unsafe { malloc(size_of::<T>()) as *mut T };
    unsafe { *p = value };
    p
}

fn test() -> usize {
    id::<Entry>(Entry::Entry(0, 0));
    let p: *mut Entry = allocate::<Entry>(Entry::Entry(22, 20));
    unsafe {
        let Entry::Entry(a, b): &Entry = &*p;
        *a + *b
    }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
    fn malloc(size: usize) -> *mut u8;
}
