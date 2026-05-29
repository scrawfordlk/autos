define i64 @main() {
entry:
  %p = call ptr @malloc(i64 1)
  store i8 42, ptr %p
  %value = load i8, ptr %p
  %widen = zext i8 %value to i64
  ret i64 %widen
}

declare ptr @malloc(i64)

