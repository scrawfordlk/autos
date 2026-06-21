enum Either<T> {
    Left(T),
    Right(T),
}

enum Container<T> {
    Content(T),
}

enum Mixed<T> {
    Mixed(Container<usize>, Container<T>),
}

enum Normal {
    Normal(usize),
}

fn test() -> usize {
    let either: Either<u8> = Either::<u8>::Right(42 as u8);
    let container_usize: Container<usize> = Container::<usize>::Content(333);
    let container: Container<Either<u8>> = Container::<Either<u8>>::Content(either);
    let mixed: Mixed<Either<u8>> = Mixed::<Either<u8>>::Mixed(container_usize, container); //  TODO: this line

    let Mixed::Mixed(c1, c2): &Mixed<Either<u8>> = &mixed;
    let Container::Content(n): &Container<usize> = c1;
    let Container::Content(e): &Container<Either<u8>> = c2;

    let wrapper: Container<usize> = match e {
        Either::Left(_) => Container::<usize>::Content(100),
        Either::Right(u1) => Container::<usize>::Content(*u1 as usize),
    };

    let Container::Content(value): Container<usize> = wrapper;

    (*n == 333) as usize + (value == 42) as usize + 40
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
    fn malloc(size: usize) -> *mut u8;
}
