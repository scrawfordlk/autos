; alloca allocating zero bytes is legal, but
; pointers are not necessarily unique then.
define i64 @main() {
entry:
  %0 = alloca i8, i64 0
  %1 = alloca i1, i64 0
  %2 = alloca i64, i64 0
  %3 = alloca ptr, i64 0
  ret i64 42
}
