declare void @puts(ptr)

define i64 @main() {
  %enum = call [16 x i8] @getstring() ; call and receive an entire array (copy?)
  %local = alloca [16 x i8] ; allocate stack space for the array
  store [16 x i8] %enum, ptr %local ; store the entire array at %local (copy?)

  %t2 = ptrtoint ptr %local to i64
  %t3 = add i64 %t2, 8
  %len.ptr = inttoptr i64 %t3 to ptr
  %len = load i64, ptr %len.ptr

  ret i64 %len
}

@msg = constant [5 x i8] c"Hell\00"

define [16 x i8] @getstring() {
  %local = alloca [16 x i8], i64 1 ; stack allocate the array

  store ptr @msg, ptr %local

  %t2 = ptrtoint ptr %local to i64
  %t3 = add i64 %t2, 8
  %len.ptr = inttoptr i64 %t3 to ptr
  store i64 5, ptr %len.ptr

  %t = load [16 x i8], ptr %local ; load entire array into register (copy?)

  ret [16 x i8] %t ; return the entire array (copy?)
}
