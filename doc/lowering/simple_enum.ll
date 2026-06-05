; Enum values are always a pointer to an array
;
; A copy is performed by allocating new memory,
; loading the ptr to the array and storing the array in new memory.
;   - when returning
;     - sret, it is simple to do (signature needs sret, copy before a ret)
;   - when assigning
;     - in assignment, copy before storing (store for variable)
;   - when passing to function as argument (thus parameter has to be copied)
;     - at beginning of function, copy before storing (store for variable)
;
; (Maybe optimisation opportunity because of Rust's move semantics/borrowing rules)
;
; A variable/parameter is therefore a pointer to a pointer to an array.
; This way, variable/parameter use is the same as before: use means load

define i64 @main() {
entry:
  ; Colour::Green('a')
  %t0 = alloca [2 x i64]
  store i64 0, ptr %t0 ; 2 stands for green
  %char = call ptr @ptr.add(ptr %t0, i64 8)
  store i8 97, ptr %char ; 97 == 'a'

  ; ... my_function(...);
  ; We know the return type is enum;
  ; 1. Caller allocates memory for the result
  ; 2. Pass the pointer where the return result will be stored int
  ; 3. After calling, %.sret will contain the return value
  %.sret = alloca [2 x i64]
  call void @my_function(ptr %.sret, ptr %t0)

  ; let colour = ...
  ; store returned enum at variable `colour`
  %colour = alloca ptr
  store ptr %.sret, ptr %colour

  ; match colour {...
  %t1 = load ptr, ptr %colour
  %discriminant = load i64, ptr %t1
  %cond1 = icmp eq i64 %discriminant, 0
  br i1 %cond1, label %then1, label %else1

then1:
  %valptr = call ptr @ptr.add(ptr %t1, i64 8)
  %val = load i64, ptr %valptr
  call void @puts(ptr @str1)
  br label %end

else1:
  %cond2 = icmp eq i64 %discriminant, 1
  br i1 %cond2, label %then2, label %else2

then2:
  call void @puts(ptr @str2)
  br label %end

else2:
  ; copy enum
  %t2 = alloca [2 x i64]
  %t3 = load [2 x i64], ptr %t1
  store [2 x i64] %t3, ptr %t2
  ; store at variable `x`
  %x = alloca ptr
  store ptr %t2, ptr %x
  call void @puts(ptr @str3)
  br label %end

end:
  %inner = call ptr @ptr.add(ptr %t1, i64 8)
  %t4 = load i8, ptr %inner
  %t5 = zext i8 %t4 to i64
  ret i64 %t5
}

declare void @puts(ptr %str)

@str1 = constant [4 x i8] c"Red\00"
@str2 = constant [5 x i8] c"Blue\00"
@str3 = constant [6 x i8] c"Green\00"


define void @my_function(ptr %.sret, ptr %param) {
  ; copy array for pass by value
  %t0 = alloca [2 x i64]
  %t1 = load [2 x i64], ptr %param
  store [2 x i64] %t1, ptr %t0
  ; store at variable `colour`
  %colour = alloca ptr
  store ptr %t0, ptr %colour

  ; match colour {....
  %t2 = load ptr, ptr %colour
  %discriminant = load i64, ptr %t2
  %cond = icmp eq i64 %discriminant, 0
  br i1 %cond, label %then, label %else

then:
  ; copy usize value to `val`
  %valvalue = load i64, ptr %t2
  %val = alloca i64
  store i64 %valvalue, ptr %val

  ; Colour::Red(42)
  %c = alloca [2 x i64]
  store i64 0, ptr %c
  %usizeptr = call ptr @ptr.add(ptr %c, i64 8)
  store i64 42, ptr %usizeptr

  ; copy to sret and return
  %c2 = load [2 x i64], ptr %c
  store [2 x i64] %c2, ptr %.sret
  ret void
  br label %end

else:
  ; copy enum
  %t3 = alloca [2 x i64]
  %t4 = load [2 x i64], ptr %t2
  store [2 x i64] %t4, ptr %t3
  ; store at variable `x`
  %x = alloca ptr
  store ptr %t3, ptr %x

  load ptr, ptr %x
  %t5 = load [2 x i64], ptr %t2
  store [2 x i64] %t5, ptr %.sret
  ret void
  br label %end

end:
  ; Colour::Green('b')
  %green = alloca [2 x i64]
  store i64 2, ptr %green ; 2 stands for green
  %char = call ptr @ptr.add(ptr %green, i64 8)
  store i8 98, ptr %char

  ; implicit return detected in codegen_function(), so copy to sret
  %retval = load [2 x i64], ptr %green
  store [2 x i64] %retval, ptr %.sret
  ret void
}

define ptr @ptr.add(ptr %p, i64 %offset) {
  %t = ptrtoint ptr %p to i64
  %t2 = add i64 %t, %offset
  %p2 = inttoptr i64 %t2 to ptr
  ret ptr %p2
}
