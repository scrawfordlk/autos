define i64 @main() {
entry:
  %cast = call i64 @cast() ; = 4
  %literal = call i64 @literal()
  %add = call i64 @add()
  %sub = call i64 @sub()
  %t0 = add i64 %cast, %literal
  %t1 = add i64 %t0, %add
  %t2 = add i64 %t1, %sub
  ret i64 %t2
}

define i64 @cast() {
entry:
  %t = add i64 260, 0
  %truncated = trunc i64 %t to i8 ; truncates, so 4
  %retvalue = zext i8 %truncated to i64
  ret i64 %retvalue
}

define i64 @literal() {
entry:
  %cmp = icmp eq i1 5, 7 ; equal if interpreted as i1 (i.e. LSB matches)
  %retvalue = zext i1 %cmp to i64
  ret i64 %retvalue ; 1
}

define i64 @add() {
entry:
  %sum = add i8 140, 130 ;  270 % 256 = 14
  %extended = zext i8 %sum to i64
  ret i64 %extended
}

define i64 @sub() {
entry:
  %diff = sub i8 100, 333 ; = 256 - 233 = 23
  %extended = zext i8 %diff to i64
  ret i64 %extended
}
