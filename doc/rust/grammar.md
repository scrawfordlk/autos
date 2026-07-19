# Grammar

## Global

```
language  -> { function | extern | enum }

function  -> signature block

signature -> [ "unsafe" ] "fn" identifier [ generic ]
             "(" [ variable { "," variable } [ "," ] ] ")" [ "->" type ]

generic   -> "<" ( T | "'" identifier [ "," "T" ] ) ">"

extern    -> "unsafe" "extern" ""C"" "{" { signature ";" } "}"

enum      -> "enum" identifier [ generic ] "{" variant "," { variant "," } "}"

variant   -> identifier [ "(" type { "," type } ")" ]

block     -> "{" { ( binding | expression [ ";" ] ) } "}"
```

## Expression

```
expression -> [ "return" [ expression ] ] | assignment

assignment -> comparison [ "=" comparison ] | comparison [ "*" "=" comparison ]

comparison -> arithmetic [ ( "==" | "!=" | "<" | ">" | "<=" | ">=" ) arithmetic ]

arithmetic -> term { ( "+" | "-" ) term }

term       -> cast { ( "*" | "/" | "%" ) cast }

cast       -> unary { "as" type }

unary      -> [ "*" | ( "&" [ "mut" ] ) ] unary | factor

factor     -> ( literal
            | identifier
            | path
            | "(" expression ")"
            | [ "unsafe" ] block
            | if
            | while
            | match )
```

## Control Flow

```
if    -> "if" expression block [ "else" [ if | block ] ]

while -> "while" expression block

match -> "match" expression "{" { arm } "}"

arm   -> pattern { "|" pattern } "=>" expression ","

path  -> identifier [ "::" "<" type ">" ] [ "::" identifier ]
         [ "(" [ expression { "," expression } [ "," ] ] ")" ]
```

## Pattern

```
binding  -> "let" variable "=" expression ";"

variable -> pattern ":" type

pattern  -> literal
          | [ "mut" ] identifier
          | identifier "::" identifier [ "(" pattern { "," pattern } [ "," ] ")" ] )
          | "_"
```

## Types & Literals

```
type       -> "u8"
            | "usize"
            | "bool"
            | "char"
            | identifier [ "<" type ">" ]
            | ( "&" [ "'" identifier ] [ "mut" ] | "*" "mut" ) type

literal    -> integer | string | character | boolean

integer    -> digit { digit }

string     -> """ { printable_character } """

character  -> "'" printable_character "'"

boolean    -> "true" | "false"

identifier -> ( letter | "_" ) { letter | digit | "_" }

letter     -> "a" | ... | "z" | "A" | ... | "Z"

digit      -> "0" | ... | "9"
```
