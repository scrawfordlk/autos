define i64 @main() {
entry:
  %val = alloca [16 x i8] ; 8 for discriminant, 8 for biggest variant (Cat with field usize)
  store i64 1, ptr %val ; store discriminant
  %t = ptrtoint ptr %val to i64
  %t2 = add i64 %t, 8
  %field = inttoptr i64 %t2 to ptr
  store i64 10, ptr %field

  %discriminant = load i64, ptr %val
  %bool = icmp eq i64 %discriminant, 2
  br i1 %bool, label %arm1, label %else1

; Animal::Seal(value) =>
arm1:
  ; initialise `value`
  %seal.value = alloca i8, i64 1
  %t10 = ptrtoint ptr %val to i64
  %t11 = add i64 %t, 8
  %t12 = inttoptr i64 %t2 to ptr
  %t13 = load i8, ptr %t12
  store i8 %t13, ptr %seal.value

  %t14 = load i8, ptr %seal.value
  %res1 = zext i8 %t14 to i64
  br label %end

else1:
  %bool2 = icmp eq i64 %discriminant, 0
  %else1res = add i64 69, 0
  br i1 %bool2, label %arm2, label %else2

; Animal::Dog =>
arm2:
  %res2 = add i64 0, 1
  br label %end

else2:
  %bool3 = icmp eq i64 %discriminant, 1
  %else2res = add i64 69, 0
  br i1 %bool3, label %arm3, label %end

; Animal::Cat(value) =>
arm3:
  ; initialise `value`
  %cat.value = alloca i64, i64 1
  %t4 = ptrtoint ptr %val to i64
  %t5 = add i64 %t, 8
  %t6 = inttoptr i64 %t2 to ptr
  %t7 = load i64, ptr %t6
  store i64 %t7, ptr %cat.value

  ; value + 32
  %t8 = load i64, ptr %cat.value
  %res3 = add i64 %t8, 32
  br label %end

end:
  %result = phi i64 [%res1, %arm1], [%res2, %arm2], [%res3, %arm3], [%else2res, %else2]
  ret i64 %result
}
