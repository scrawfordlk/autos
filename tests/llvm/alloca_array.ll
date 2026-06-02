define i64 @main() {
entry:
  %arr = alloca [3 x i8]

  store i8 10, ptr %arr

  %t0 = ptrtoint ptr %arr to i64
  %t1 = add i64 %t0, 2
  %p1 = inttoptr i64 %t1 to ptr
  store i8 32, ptr %p1

  %r0 = load i8, ptr %arr
  %r1 = load i8, ptr %p1
  %a = zext i8 %r0 to i64
  %b = zext i8 %r1 to i64

  %sum = add i64 %a, %b
  ret i64 %sum
}
