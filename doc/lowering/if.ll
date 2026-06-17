define i64 @main() {
entry:
  %var = alloca i64, i64 1              ; create space for expression result ;; optional if it returns non-unit
  br i1 0, label %then, label %else     ; if-else semantics
then:                                   ; then branch:
  %t1 = add i64 0, 42
  store i64 %t1, ptr %var
  br label %end                         ; jump to end-label
else:                                   ; else branch:
  %t2 = add i64 0, 69
  store i64 %t2, ptr %var               ;; optional if it returns non-unit
  br label %end                         ; jump to else-label
end:
  %result = load i64, ptr %var          ;; optional if it returns non-unit
  ret i64 %result
}
