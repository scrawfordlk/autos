# Grammar of LLLVM

## Global

```text
language    -> { string | definition | declaration }

string      -> global "=" "constant"
               "[" constant "x" "i8" "]" "c"" { printable } """

definition  -> "define" type global
               "(" [ type local { "," type local } ] ")"
               "{" { block } "}"

declaration -> "declare" type global "(" [ type { "," type } ] ")"

block       -> label { instruction }
```

## Instructions

```text
instruction -> terminator | assignment | store | call

terminator  -> return | branch

return      -> "ret" ( "void" | type value )

branch      -> "br" "label" local
             | "br" "i1" value "," "label" local "," "label" local

assignment  -> local "=" operation

operation   -> binary
             | icmp
             | cast
             | alloca
             | load
             | call

store       -> "store" type value "," "ptr" value

binary      -> ( "add" | "sub" | "mul" | "udiv" | "urem" ) type value "," value

icmp        -> "icmp" comparison type value "," value

comparison  -> "eq" | "ne" | "ugt" | "ult" | "uge" | "ule"

cast        -> ( "zext" | "trunc" | "ptrtoint" | "inttoptr" ) type value "to" type

call        -> "call" type global "(" [ type value { "," type value } ] ")"

alloca      -> "alloca" type "," "i64" constant

load        -> "load" type "," "ptr" value
```

## Types, Literals & Identifiers

```text
global     -> "@" identifier

local      -> "%" identifier

label      -> identifier ":"

type       -> integer | "void" | "ptr"

integer    -> "i1" | "i8" | "i64"

value      -> constant | global | local

constant   -> digit { digit }

identifier -> identchar { identchar }

identchar  -> letter | digit | "_" | "." | "$"

letter     -> "a" | ... | "z" | "A" | ... | "Z"

digit      -> "0" | ... | "9"
```
