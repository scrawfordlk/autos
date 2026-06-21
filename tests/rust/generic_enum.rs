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
    let mut mixed: Mixed<Either<u8>> = Mixed::<Either<u8>>::Mixed(container_usize, container);

    let mut nested: Either<Container<Normal>> =
        Either::<Container<Normal>>::Left(Container::<Normal>::Content(Normal::Normal(0)));
    let container: Container<Normal> = f::<Container<Normal>>(
        &mut nested,
        Container::<Normal>::Content(Normal::Normal(77)),
        Container::<Normal>::Content(Normal::Normal(66)),
    );
    let Container::Content(Normal::Normal(a)): Container<Normal> = container;
    let b: usize = match &nested {
        Either::Left(Container::Content(Normal::Normal(value))) => *value,
        _ => 1000,
    };

    let Mixed::Mixed(c1, c2): &mut Mixed<Either<u8>> = &mut mixed;
    let Container::Content(n): &Container<usize> = c1; // coerce reference
    let Container::Content(e): &mut Container<Either<u8>> = c2;

    let wrapper: Container<usize> = match e {
        Either::Left(_) => Container::<usize>::Content(100),
        Either::Right(u1) => Container::<usize>::Content(*u1 as usize),
    };

    let k: u8 = f::<u8>(e, 42 as u8, 13 as u8);
    let j: u8 = match e {
        Either::Right(val) => *val,
        _ => 255 as u8,
    };

    let Container::Content(value): Container<usize> = if 42 == 42 {
        wrapper
    } else {
        Container::<usize>::Content(100)
    };

    (*n == 333) as usize
        + (value == 42) as usize
        + (k as usize == 13) as usize
        + (j as usize == 42) as usize
        + (a == 66) as usize
        + 37
}

fn f<T>(either: &mut Either<T>, value: T, other: T) -> T {
    match either {
        Either::Right(x) => {
            *x = value;
            other
        },
        Either::Left(x) => {
            *x = value;
            other
        },
    }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
