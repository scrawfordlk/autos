define i64 @main() {
entry:
  %var = alloca i64, i64 1                          ; let x: usize
  store i64 0, ptr %var                             ; = 0;
while.entry:
  %t1 = load i64, ptr %var                          ; get value of x
  %t2 = icmp ne i64 %t1, 10                         ; while x != 10
  br i1 %t2, label %while.body, label %while.end    ; if true execute body, else skip body
while.body:                                         ; {
  %t3 = load i64, ptr %var                          ;
  %t4 = add i64 %t3, 1                              ;
  store i64 %t4, ptr %var                           ; x = x + 1
  br label %while.entry                             ; } (jump back to condition)
while.end:
  %t5 = load i64, ptr %var
  ret i64 %t5                                       ; return x
}
