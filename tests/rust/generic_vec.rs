fn test() -> usize {
    let mut matrix: Vec<Vec<Value<char>>> = create_matrix::<Value<char>>(3);

    let mut row_idx: usize = 0;
    while row_idx < vec_len::<Vec<Value<char>>>(&matrix) {
        let row: &mut Vec<Value<char>> = vec_at_mut::<Vec<Value<char>>>(&mut matrix, row_idx);
        let mut col_idx: usize = 0;
        while col_idx < 3 {
            let c: char = if row_idx == col_idx { '*' } else { '.' };
            vec_push::<Value<char>>(row, Value::<char>::Value(Coords::Coords(row_idx, col_idx), c));
            col_idx = col_idx + 1
        }
        row_idx = row_idx + 1;
    }

    let mut non_diagonal: usize = 0;
    row_idx = 0;
    while row_idx < vec_len::<Vec<Value<char>>>(&matrix) {
        let row: &Vec<Value<char>> = vec_at::<Vec<Value<char>>>(&matrix, row_idx);
        let mut col_idx: usize = 0;
        while col_idx < vec_len::<Value<char>>(row) {
            if row_idx != col_idx {
                let Value::Value(Coords::Coords(x, y), c): &Value<char> = vec_at::<Value<char>>(row, col_idx);
                if or(*x != row_idx, *y != col_idx) {
                    unsafe { exit(11) }
                }
                non_diagonal = non_diagonal + *c as usize;
            }
            col_idx = col_idx + 1;
        }
        row_idx = row_idx + 1;
    }

    let mut diagonal: usize = 0;
    let mut i: usize = 0;
    while i < 3 {
        let row: &Vec<Value<char>> = vec_at::<Vec<Value<char>>>(&matrix, i);
        let Value::Value(Coords::Coords(x, y), c): &Value<char> = vec_at::<Value<char>>(row, i);
        if or(*x != i, *y != i) {
            unsafe { exit(11) }
        }
        diagonal = diagonal + *c as usize;
        i = i + 1;
    }

    (diagonal / 3 == '*' as usize) as usize + (non_diagonal / (9 - 3) == '.' as usize) as usize + 40
}

fn create_matrix<T>(size: usize) -> Vec<Vec<T>> {
    let mut rows: Vec<Vec<T>> = vec_new::<Vec<T>>();
    let mut i: usize = 0;
    while i < size {
        vec_push::<Vec<T>>(&mut rows, vec_new::<T>());
        i = i + 1;
    }
    rows
}

enum Value<T> {
    Value(Coords, T),
}

enum Coords {
    Coords(usize, usize),
}

enum Vec<T> {
    /// start, length, capacity
    Vec(*mut T, usize, usize),
}

/// Create an empty vector.
fn vec_new<T>() -> Vec<T> {
    vec_with_capacity::<T>(10)
}

fn vec_with_capacity<T>(initial_capacity: usize) -> Vec<T> {
    let capacity: usize = max(initial_capacity, 1);
    let ptr: *mut T = unsafe { alloc::<T>(capacity) };
    Vec::<T>::Vec(ptr, 0, capacity)
}

fn vec_ptr<T>(Vec::Vec(ptr, _, _): &Vec<T>) -> *mut T {
    *ptr
}

fn vec_len<T>(Vec::Vec(_, len, _): &Vec<T>) -> usize {
    *len
}

fn vec_capacity<T>(Vec::Vec(_, _, capacity): &Vec<T>) -> usize {
    *capacity
}

fn vec_accomodate_extra_space<T>(vec: &mut Vec<T>, space: usize) {
    let len: usize = vec_len::<T>(vec);
    let capacity: usize = vec_capacity::<T>(vec);
    if capacity < len + space {
        let Vec::Vec(ptr, len_ref, capacity_ref): &mut Vec<T> = vec;
        while len + space > *capacity_ref {
            *capacity_ref = *capacity_ref * 2;
        }
        let new_ptr: *mut T = unsafe { alloc::<T>(*capacity_ref) };
        unsafe { memcopy::<T>(new_ptr, *ptr, *len_ref) };
        *ptr = new_ptr;
    }
}

fn vec_push<T>(vec: &mut Vec<T>, value: T) {
    vec_accomodate_extra_space::<T>(vec, 1);
    let Vec::Vec(ptr, len, _): &mut Vec<T> = vec;
    unsafe { *ptr_add::<T>(*ptr, *len) = value };
    *len = *len + 1;
}

fn vec_get<T>(vec: &Vec<T>, index: usize) -> Option<&T> {
    if index >= vec_len::<T>(vec) {
        Option::<&T>::None
    } else {
        let ptr: *mut T = ptr_add::<T>(vec_ptr::<T>(vec), index);
        unsafe { Option::<&T>::Some(&*ptr) }
    }
}

fn vec_get_mut<T>(vec: &mut Vec<T>, index: usize) -> Option<&mut T> {
    if index >= vec_len::<T>(vec) {
        Option::<&mut T>::None
    } else {
        let ptr: *mut T = ptr_add::<T>(vec_ptr::<T>(vec), index);
        unsafe { Option::<&mut T>::Some(&mut *ptr) }
    }
}

fn vec_at<T>(vec: &Vec<T>, index: usize) -> &T {
    if index >= vec_len::<T>(vec) {
        unsafe { exit(7) }
    }
    unwrap::<&T>(vec_get::<T>(vec, index))
}

fn vec_at_mut<T>(vec: &mut Vec<T>, index: usize) -> &mut T {
    if index >= vec_len::<T>(vec) {
        unsafe { exit(9) }
    }
    unwrap::<&mut T>(vec_get_mut::<T>(vec, index))
}

unsafe fn memcopy<T>(dest: *mut T, src: *mut T, n: usize) {
    let byte_count: usize = n * size_of::<T>();
    let dest_u8: *mut u8 = dest as *mut u8;
    let src_u8: *mut u8 = src as *mut u8;
    let mut i: usize = 0;
    while i < byte_count {
        unsafe { *ptr_add::<u8>(dest_u8, i) = *ptr_add::<u8>(src_u8, i) };
        i = i + 1;
    }
}

fn ptr_add<T>(ptr: *mut T, n: usize) -> *mut T {
    (ptr as usize + n * size_of::<T>()) as *mut T
}

unsafe fn alloc<T>(count: usize) -> *mut T {
    // count == 10, size_of::<T> = 32
    unsafe {
        let p: *mut u8 = malloc(size_of::<T>() * count);
        if p as usize == 0 {
            exit(12)
        }
        p as *mut T
    }
}

enum Option<T> {
    Some(T),
    None,
}

fn unwrap<T>(opt: Option<T>) -> T {
    match opt {
        Option::Some(value) => value,
        Option::None => unsafe { exit(8) },
    }
}

fn max(n: usize, m: usize) -> usize {
    if n > m { n } else { m }
}

fn or(a: bool, b: bool) -> bool {
    a as usize + b as usize > 0
}

fn main() {
    unsafe { exit(test()) }
}

fn exit_process(code: usize) -> ! {
    unsafe { exit(code) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
    fn malloc(size: usize) -> *mut u8;
    fn open(path: *mut u8, flags: usize, mode: usize) -> usize;
    fn write(fd: usize, buf: *mut u8, count: usize) -> usize;
    fn read(fd: usize, buf: *mut u8, count: usize) -> usize;
}
