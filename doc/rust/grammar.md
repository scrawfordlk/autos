# Grammar

## Global

```text
language  -> { item }

item      -> enum
           | function
           | "unsafe" ( extern | function )

function  -> signature block

signature -> "fn" identifier [ generic ]
             "(" [ variable { "," variable } [ "," ] ] ")" [ "->" type ]

generic   -> "<" ( "T" | "'" identifier [ "," "T" ] ) ">"

extern    -> "extern" ""C"" "{" { signature ";" } "}"

enum      -> "enum" identifier [ generic ] "{" variant "," { variant "," } "}"

variant   -> identifier [ "(" type { "," type } ")" ]

block     -> "{" { binding | expression ";" } [ expression ] "}"
```

## Expression

```text
expression -> "return" [ expression ] | assignment

assignment -> comparison [ "=" assignment ]

comparison -> arithmetic
              [ ( "==" | "!=" | "<" | ">" | "<=" | ">=" ) arithmetic ]

arithmetic -> term { ( "+" | "-" ) term }

term       -> cast { ( "*" | "/" | "%" ) cast }

cast       -> unary { "as" type }

unary      -> ( "*" | "&" [ "mut" ] ) unary | factor

factor     -> literal
            | path
            | "(" expression ")"
            | [ "unsafe" ] block
            | if
            | while
            | match

path       -> identifier [ args | "::" pathsuffix ]

pathsuffix -> identifier [ args ]
            | "<" type ">" ( args | "::" identifier [ args ] )

args       -> "(" [ expression { "," expression } [ "," ] ] ")"
```

## Control Flow

```text
if    -> "if" expression block [ "else" [ if | block ] ]

while -> "while" expression block

match -> "match" expression "{" { arm } "}"

arm   -> pattern { "|" pattern } "=>" expression ","
```

## Pattern

```text
binding  -> "let" variable "=" expression ";"

variable -> pattern ":" type

pattern  -> literal
          | "mut" identifier
          | identifier [ "::" identifier
              [ "(" pattern { "," pattern } [ "," ] ")" ] ]
          | "_"
```

## Types & Literals

```text
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
