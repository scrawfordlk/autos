# Lowering of Generics

## Generic Functions
### Declaration
```rust
fn f<T>() {
  // ...
}
```

Instantiation requires turbofish (`::<T>`) syntax, so that type parameter is explicit:

```rust
f::<usize>()
```

## Generic Enums
### Declaration
```rust
enum Box<T> {
  Ptr(*mut T)
}
```
### Usage
Instantiating enums also requires turbofish syntax to avoid having to infer the type of the type parameter:
```rust
let p: *mut T = ...;
let box: Box<usize> = Box::<usize>::Ptr(p);
```

## Special Cases to check
```rust
fn f<T>(value: T) {
  f::<List<T>>(new::<List<T>>())
}
```
Infinite recursion due to creating a new type each time when recursing.
- Fix through detection: Maybe count the number of recursions and stop after a hard limit?

```rust
enum List<T> {
  Inner(*mut List<List<T>>)
}
```
Infinite recursion despite indirection due to infinite size when instantiating the type with a value.
- Fix through limitation: Do not allow usage of same type inside an enum

## Built-in generic functions
- `size_of<T>()` - returns the size of the type parameter
  - Easiest implementation is to have the compiler check for `size_of()` when generating code for a function call and then inlining the value as a literal. This is the easiest (no need to emit entire generic functions, type size is computed at compile time) and most efficient (removal of unnecessary function calls, compile-time).

## Implementation
