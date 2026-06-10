# LLVM-IR

## 1. What a program looks like

An LLVM-IR file is a sequence of:

- global constants
- function declarations (`declare`)
- function definitions (`define`)

Example:

```llvm
@.msg = constant [5 x i8] c"hello"

declare i64 @read()

define i64 @main() {
entry:
  %x = call i64 @read()
  ret i64 %x
}
```

## 2. Names, registers, and symbols

- `%name` is a local variable, often referred to as virtual register. Register allocation is handled by the backend, therefore there are infinite virtual registers.
- `@name` is a global symbol (global data or function name).
- Labels (like `entry:`) mark basic blocks inside a function.

## 3. Types

- `i1`, `i8`, `i64`: integers with width of 1, 8 or 64 respectively
- `void`: no return value
- `ptr`: pointer
- `[N x <type>]`: array with `N` elements of type `<type>`

Every instruction is explicitly typed. For example, arithmetic states the operand type:

```llvm
%sum = add i64 %a, %b
```

## 4. Constants and literals

Supported literals:

- integer literal: `0`, `42`
- string literal for constant byte arrays: `c"hello"`

Global constants use the form:

```llvm
@name = constant <type> <literal>
```

Example:

```llvm
@.msg = constant [6 x i8] c"hello\00"
```

## 5. Functions

### Declaration (`declare`)

Declares a function signature without a body (provided externally):

```llvm
declare i64 @read()
declare void @exit(i64)
```

Declarable functions are:

- `open()`
- `read()`
- `write()`
- `malloc()`
- `exit()`

### Definition (`define`)

Defines a function:

```llvm
define i64 @increment(i64 %x) {
entry:
  %y = add i64 %x, 1
  ret i64 %y
}
```

## 6. Basic blocks and control flow

A function body is made of basic blocks. A block is:

1. One label (`entry:`) at the beginning,
2. $n$ normal instructions,
3. one terminator instruction (`ret` or `br`) at the end.

(A terminator instruction does not have to be at the end of a block. It may also appear anywhere else.)

Branch instruction (`br`):

- unconditional jump:

  ```llvm
  br label %done
  ```

- conditional jump:

  ```llvm
  br i1 %cond, label %then, label %else
  ```

Here, `%name` denotes a label, not a virtual register with a value.

## 7. Instruction set in this subset

### Arithmetic

- `add`, `sub`, `mul`, `udiv`, `urem`

Example:

```llvm
%r = mul i64 21, 2
```

### Comparison

- `icmp` compares two integer values and returns a value of type `i1`.
- The result depends on the comparison operation specified after `icmp`
  - `eq`
  - `ne`
  - `ugt`
  - `ult`
  - `uge`
  - `ule`

Example:

```llvm
%is_less = icmp ult i64 %a, %b
```

### Casting

Casting can be done by zero-extending a type with a smaller bitwidth to a type with a larger bitwidth, by truncating a type with a larger bitwidth to a type with a smaller bitwidth, by interpreting an integer as a pointer or by interpreting a pointer as an integer.

Examples:

```llvm
%large_value = add i64 10000000, 0
%truncated = trunc i64 %large_value to i1
```

```llvm
%small_value = add i1 1, 0
%extended = zext i1 %small_value to i64
```

```llvm
%p.as.int = ptrtoint ptr %p1 to i64
%new.pointer = add i64 %p.as.int, 8
%p2 = inttoptr i64 %new.pointer to ptr
```

As one can see, pointer arithmetic can be performed in this way, making the infamous `getelementptr` instruction redundant.

### Function call

Calls another function and optionally captures the return value:

```llvm
%v = call i64 @read()
call void @write_i64(i64 %v)
```

### Return

- `ret void`
- `ret <type> <value>`

Examples:

```llvm
ret void
ret i64 %r
```

## 8. Static Single Assignment (SSA) form

LLVM-IR follows Static Single Assignment (SSA) form:

1. All virtual registers may only be assigned to once (their contents can't be mutated).
2. All uses of virtual registers may only refer to a single definition.

So this is valid:

```llvm
%x0 = add i64 %x, 1
%x1 = mul i64 %x0, 2
```

while this violates SSA and hence is invalid:

```llvm
%x = add i64 %y, 1
%x = mul i64 %x, 2
```
