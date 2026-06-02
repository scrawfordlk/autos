define i64 @main() {
entry:
  %c1 = load i8, ptr @.str
  %widen1 = zext i8 %c1 to i64

  %t0 = ptrtoint ptr @.str to i64
  %t1 = add i64 %t0, 1
  %ptr2 = inttoptr i64 %t1 to ptr
  %c2 = load i8, ptr %ptr2
  %widen2 = zext i8 %c2 to i64

  %t2 = ptrtoint ptr @.str to i64
  %t3 = add i64 %t2, 2
  %ptr3 = inttoptr i64 %t3 to ptr
  %c3 = load i8, ptr %ptr3
  %widen3 = zext i8 %c3 to i64

  %.52 = sub i64 %widen1, %widen2
  %result = sub i64 %.52, %widen3
  ret i64 %result
}

@.str = constant [3 x i8] c"a-\0A"
