define i64 @main() {
entry:
  call void @f()
  ret i64 4
}

define void @f() {
entry:
  call void @exit(i64 42)
  ret void
}

declare void @exit(i64)
