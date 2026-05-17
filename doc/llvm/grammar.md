# Grammar of LLVM-IR

```
llvm       -> { string | function }

string     -> global "=" "constant" array "c"" { printable_character } "\""

function   -> "define" type global  parameters "{" blocks "}"

parameters -> "(" [ type register { "," type register } ] ")"

global     -> "@" identifier
```

```
blocks      -> { block }

block       -> identifier ":" { instruction }

register    -> "%" identifier

instruction -> return | branch | assignment | store | call

return      -> "ret" ( "void" | type value )

branch      -> "br" "label" register
             | "br" "i1" value "," "label" register "," "label" register

assignment  -> register "=" operation

operation   -> binary
             | icmp
             | cast
             | call
             | alloca
             | load
             | gep

store       -> "store" type value "," "ptr" value

binary      -> ( "add" | "sub" | "mul" | "udiv" | "urem" ) type value "," value

icmp        -> "icmp" ( "eq" | "ne" | "ugt" | "ult" | "uge" | "ule" ) type value "," value

cast        -> ( "zext" | "trunc" ) type value "to" type

call        -> "call" type global "(" [ type value { "," type value } ] ")"

alloca      -> "alloca" type "," "i64" number

load        -> "load" type "," "ptr" value

gep         -> "getelementptr" type "," "ptr" value "," type value { "," type value }

type        -> integer | "void" | "ptr" | "[" number "x" type "]"

integer     -> "i64" | "i32" | "i8" | "i1"

value       -> register | number | global

literal     -> number | array

array       -> "[" [ type literal { "," type literal } ] "]"

number      -> [ "-" ] digit { digit }

identifier  -> ( letter | "_" | "." ) { letter | digit | "_" | "." }
```
