# Grammar of LLLVM

## Global

```text
language    -> { string | definition | declaration }

string      -> global "=" "constant"
               "[" number "x" "i8" "]" "c"" { printable_character } """

definition  -> "define" type global
               "(" [ type local { "," type local } ] ")"
               "{" { block } "}"

declaration -> "declare" type global "(" [ type { "," type } ] ")"

block       -> label { instruction }
```

## Instructions

```text
instruction -> return | branch | assignment | store | call

return      -> "ret" ( "void" | type value )

branch      -> "br" "label" local
             | "br" "i1" value "," "label" local "," "label" local

assignment  -> local "=" operation

operation   -> binary
             | icmp
             | cast
             | call
             | alloca
             | load

store       -> "store" type value "," "ptr" value

binary      -> ( "add" | "sub" | "mul" | "udiv" | "urem" ) type value "," value

icmp        -> "icmp" comparison type value "," value

comparison  -> "eq" | "ne" | "ugt" | "ult" | "uge" | "ule"

cast        -> ( "zext" | "trunc" | "ptrtoint" | "inttoptr" ) type value "to" type

call        -> "call" type global "(" [ type value { "," type value } ] ")"

alloca      -> "alloca" type "," "i64" number

load        -> "load" type "," "ptr" value
```

## Types, Literals & Identifiers

```text
global      -> "@" identifier

local      -> "%" identifier

label       -> identifier ":"

type       -> integer | "void" | "ptr"

integer    -> "i64" | "i8" | "i1"

value      -> local | number | global

number     -> digit { digit }

identifier -> identchar { identchar }

identchar  -> "a" | ... | "z" | "A" | ... | "Z" | digit | "_" | "." | "$"

digit      -> "0" | ... | "9"
```
