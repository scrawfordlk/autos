@msg = constant [5 x i8] c"Hello"

define i64 @main() {
  ; allocate, store and load a struct with pointer and length
  %string.ptr = alloca [16 x i8]
  store ptr @msg, ptr %string.ptr
  %string.len = getelementptr [16 x i8], ptr %string.ptr, i32 0, i32 8
  store i64 5, ptr %string.len
  %string = load [16 x i8], ptr %string.ptr

  ; pass above array into function and get back another array
  %cut = call [16 x i8] @cut([16 x i8] %string)
  %cut.ptr = alloca [16 x i8]
  store [16 x i8] %cut, ptr %cut.ptr

  %len.ptr = getelementptr [16 x i8], ptr %cut.ptr, i32 0, i32 8
  %len = load i64, ptr %len.ptr

  ret i64 %len
}

; cuts string by one char at the end
define [16 x i8] @cut([16 x i8] %input) {
  ; copy input
  %copy = alloca [16 x i8]
  store [16 x i8] %input, ptr %copy

  ; cut off last element by reducing length by 1
  %len.ptr = getelementptr [16 x i8], ptr %copy, i32 0, i32 8
  %len = load i64, ptr %len.ptr
  %newlen = sub i64 %len, 1
  store i64 %newlen, ptr %len.ptr

  %array = load [16 x i8], ptr %copy
  ret [16 x i8] %array
}
