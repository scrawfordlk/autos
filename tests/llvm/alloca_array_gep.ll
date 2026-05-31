define i64 @main() {
entry:
  %arr = alloca [4 x i8], i64 1
  %p0 = getelementptr [4 x i8], ptr %arr, i64 0, i64 0
  store i8 10, ptr %p0
  %p1 = getelementptr [4 x i8], ptr %arr, i64 0, i64 1
  store i8 32, ptr %p1
  %r0 = load i8, ptr %p0
  %r1 = load i8, ptr %p1
  %a = zext i8 %r0 to i64
  %b = zext i8 %r1 to i64
  %sum = add i64 %a, %b
  ret i64 %sum
}
