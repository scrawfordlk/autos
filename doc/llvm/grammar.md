# Grammar of LLVM-IR

## Global

```
llvm     -> { string | function }

string   -> global "=" "constant" array "c"" { printable_character } """

function -> "define" type global
            "(" [ type local { "," type local } ] ")"
            "{" "entry" ":" { instruction } { identifier ":" { instruction } } "}"

global   -> "@" identifier
```

## Instructions

```
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

icmp        -> "icmp" ( "eq" | "ne" | "ugt" | "ult" | "uge" | "ule" ) type value "," value

cast        -> ( "zext" | "trunc" | ptrtoint | inttoptr ) type value "to" type

call        -> "call" type global "(" [ type value { "," type value } ] ")"

alloca      -> "alloca" type

load        -> "load" type "," "ptr" value
```

## Types, Literals & Identifiers

```
local      -> "%" identifier

type       -> integer | "void" | "ptr" | "[" number "x" integer ]"

integer    -> "i64" | "i8" | "i1"

value      -> local | number | global

number     -> digit { digit }

identifier -> ( letter | "_" | "." ) { letter | digit | "_" | "." }

letter     -> "a" | ... | "z" | "A" | ... | "Z"

digit      -> "0" | ... | "9"
```
