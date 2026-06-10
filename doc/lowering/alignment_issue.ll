; compile and run with clang
; it segfaults when storing (see marked store)
; my guess as to why is because of memory alignment

define i64 @test() {
entry:
  %t0 = alloca [3 x i64]
  store i64 0, ptr %t0

  ; increment by 8
  %t1 = ptrtoint ptr %t0 to i64
  %t2 = add i64 %t1,8
  %t3 = inttoptr i64 %t2 to ptr
  store i64 134, ptr %t3

  ; increment by 1
  %t4 = ptrtoint ptr %t3 to i64
  %t5 = add i64 %t4,1
  %t6 = inttoptr i64 %t5 to ptr
  store i8 65, ptr %t6

  ; increment by 8
  %t7 = ptrtoint ptr %t6 to i64
  %t8 = add i64 %t7,1
  %t9 = inttoptr i64 %t8 to ptr

  store i8 48, ptr %t9

  ; increment by 8
  %t10 = ptrtoint ptr %t9 to i64
  %t11 = add i64 %t10,8
  %t12 = inttoptr i64 %t11 to ptr

  store i64 4, ptr %t12 ; causes segfault

  %t13 = load [3 x i64], ptr %t0
  %t14 = alloca [3 x i64]
  store [3 x i64] %t13, ptr %t14
  %t15 = alloca [3 x i64]
  store i64 1, ptr %t15
  %t16 = ptrtoint ptr %t15 to i64
  %t17 = add i64 %t16,8
  %t18 = inttoptr i64 %t17 to ptr
  store i64 233, ptr %t18
  %t19 = ptrtoint ptr %t18 to i64
  %t20 = add i64 %t19,8
  %t21 = inttoptr i64 %t20 to ptr
  store i64 255, ptr %t21
  %t22 = ptrtoint ptr %t21 to i64
  %t23 = add i64 %t22,1
  %t24 = inttoptr i64 %t23 to ptr
  store i8 10, ptr %t24
  %t25 = load [3 x i64], ptr %t15
  %t26 = alloca [3 x i64]
  store [3 x i64] %t25, ptr %t26
  %t27 = alloca [3 x i64]
  store i64 0, ptr %t27
  %t28 = ptrtoint ptr %t27 to i64
  %t29 = add i64 %t28,8
  %t30 = inttoptr i64 %t29 to ptr
  store i64 120, ptr %t30
  %t31 = ptrtoint ptr %t30 to i64
  %t32 = add i64 %t31,1
  %t33 = inttoptr i64 %t32 to ptr
  store i8 53, ptr %t33
  %t34 = ptrtoint ptr %t33 to i64
  %t35 = add i64 %t34,1
  %t36 = inttoptr i64 %t35 to ptr
  store i8 48, ptr %t36
  %t37 = ptrtoint ptr %t36 to i64
  %t38 = add i64 %t37,8
  %t39 = inttoptr i64 %t38 to ptr
  store i64 7, ptr %t39
  %t40 = load [3 x i64], ptr %t27
  %t41 = alloca [3 x i64]
  store [3 x i64] %t40, ptr %t41
  %t42 = alloca [3 x i64]
  store i64 2, ptr %t42
  %t43 = ptrtoint ptr %t42 to i64
  %t44 = add i64 %t43,8
  %t45 = inttoptr i64 %t44 to ptr
  store ptr %t41, ptr %t45
  %t46 = ptrtoint ptr %t45 to i64
  %t47 = add i64 %t46,8
  %t48 = inttoptr i64 %t47 to ptr
  store i64 1, ptr %t48
  %t49 = load [3 x i64], ptr %t42
  %t50 = alloca [3 x i64]
  store [3 x i64] %t49, ptr %t50
  ret i64 42
}

define i64 @main() {
entry:
  call i64 @test()
  call void @exit(i64 42)
  ret i64 0
}
declare void @exit(i64)
