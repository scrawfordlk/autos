define i64 @main() {
entry:
  %result = call i64 @sum2(i64 40, i64 2)
  ret i64 %result
}

define i64 @sum2(i64 %a, i64 %b) {
entry:
  %sum = add i64 %a, %b
  ret i64 %sum
}
