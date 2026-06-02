; NOTE: uses [2 x i64], this leads to much more concise generated code (because it copies double words instead of bytes)

@msg = constant [5 x i8] c"Hello"

define i64 @main() {
  ; allocate, store and load a struct with pointer and length
  %string.ptr = alloca [2 x i64]
  store ptr @msg, ptr %string.ptr
  %string.len = getelementptr [2 x i64], ptr %string.ptr, i32 0, i32 8
  store i64 5, ptr %string.len
  %string = load [2 x i64], ptr %string.ptr

  ; pass above array into function and get back another array
  %cut = call [2 x i64] @cut([2 x i64] %string)
  %cut.ptr = alloca [2 x i64]
  store [2 x i64] %cut, ptr %cut.ptr

  %len.ptr = getelementptr [2 x i64], ptr %cut.ptr, i32 0, i32 8
  %len = load i64, ptr %len.ptr

  ret i64 %len
}

; cuts string by one char at the end
define [2 x i64] @cut([2 x i64] %input) {
  ; copy input
  %copy = alloca [2 x i64]
  store [2 x i64] %input, ptr %copy

  ; cut off last element by reducing length by 1
  %len.ptr = getelementptr [2 x i64], ptr %copy, i32 0, i32 8
  %len = load i64, ptr %len.ptr
  %newlen = sub i64 %len, 1
  store i64 %newlen, ptr %len.ptr

  %array = load [2 x i64], ptr %copy
  ret [2 x i64] %array
}
