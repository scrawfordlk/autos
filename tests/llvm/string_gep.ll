define i64 @main() {
entry:
  %p = getelementptr [8 x i8], ptr @.str, i64 0, i64 1
  %c = load i8, ptr %p
  %widen = zext i8 %c to i64
  ret i64 %widen
}

@.str = constant [8 x i8] c"\10*<-Here"
