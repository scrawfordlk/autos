define i64 @main() {
entry:
  %p = alloca i64
  %addr = ptrtoint ptr %p to i64
  %p2 = inttoptr i64 %addr to ptr
  store i64 42, ptr %p2
  %v = load i64, ptr %p
  ret i64 %v
}
