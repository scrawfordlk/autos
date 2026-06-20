# Language Description

## Top-level

The only top-level items are:

- `fn`

  ```rust
  fn my_function() { ... }
  ```

- `enum` with tuple variants

  ```rust
  enum MyEnum {
    VariantA,
    VariantB(usize),
  }
  ```

No structs, impl blocks, traits, modules, macros.

## Types

- `usize`
- `u8`
- `char`
- `&str`
- user-defined tuple enums
- references: `&T`, `&mut T`
- raw mutable pointers: `*mut T`
- at most one generic type parameter (per function/enum)

## Literals and comments

You can use:

- integer literals are type `usize` (`u8` requires explicit casting)
- char literals (`char`)
- string literals (`&str`)

There are only line comments:

```rust
// comment
```

## Variables

Type inference is not supported. Hence, all types need to be written explicitly:

```rust
let s: &str = "Hello World";
let mut y: usize = 2;
let n: u8 = 42 as u8;
```

Assignments (for mutable variables) are straightforward:

```rust
let mut y: usize = 0;
y = y + 1;
let ptr: &mut = &mut y;
*ptr = 42;
```

## Operators

- arithmetic: `+`, `-`, `*`, `/`, `%`
- comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- unary operator: `-`, `*`, borrow (`&`, `&mut`)
- cast: `as`

## Control flow

- `if` / `else`

  ```rust
  let x: char = if b { 'a' } else { 'b' };
  ```

- `while`

  ```rust
  let mut i = 0;
  while i < 10 {
    i = i + 1;
  }
  ```

- `match`

  ```rust
  let message: char = match my_enum {
    MyEnum::VariantA => 'A',
    MyEnum::VariantB(value) => value as char,
  }
  ```

- `return`

  ```rust
  fn f(a: usize) -> usize {
    if a < 0 {
      return 0;
    }
    ...
  }
  ```

## Generics

Generic functions and enums are supported with some limitations:

- A function/enum can at most have one generic type parameter
- The type parameter must be called `T`)
- A generic function cannot call itself (In the near future this limitation will be lifted)
- All instantiations require turbofish (`::<T>`) syntax

Example:

```rust
enum Option<T> {
    Some(T),
    None
}

fn is_some<T>(opt: &Option<T>) => bool {
    match opt {
        Option::Some(_) => true,
        _ => false
    }
}

fn some<T>(value: T) -> Option<T> {
    Option::<T>::Some(value)
}

fn f() -> usize {
    let opt: Option<usize> = some::<usize>(42);
    if is_some::<usize>(opt) {
        42
    } else {
        0
    }
}

```

## Builtin Functions

There are currently three builtin functions: The `&str` functions
`str::as_ptr()` and `str::len()`, as well as the generic function `size_of<T>()`.
The former are needed to be able to work with `&str`, the latter is useful
for generic data structures and functions.

## I/O

For I/O, use an extern block:

```rust
unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn exit(code: usize) -> !;
    fn open(path: *mut u8, flags: usize, mode: usize) -> usize;
    fn write(fd: usize, buf: *mut u8, count: usize) -> usize;
    fn read(fd: usize, buf: *mut u8, count: usize) -> usize;
}
```

Strictly speaking, only these five functions are supported, however in theory, any function in libc can be called this way.
