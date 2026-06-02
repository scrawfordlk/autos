define i64 @main() {
entry:
  %var = alloca i64
  store i64 0, ptr %var
  %res = call i64 @sum2(i64 38, i64 2)
  %t1 = load i64, ptr %var
  %t2 = add i64 %res, %t1
  store i64 %t2, ptr %var
  call void @add2(ptr %var)
  %result = load i64, ptr %var
  ret i64 %result
}

define i64 @sum2(i64 %a, i64 %b) {
entry:
  %sum = add i64 %a, %b
  ret i64 %sum
}

define void @add2(ptr %var) {
entry:
  %value = load i64, ptr %var
  %sum = add i64 %value, 2
  store i64 %sum, ptr %var
  ret void
}
