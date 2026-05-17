# Grammar

## Top-level definitions

```
language -> { function | enum }

function -> [ "unsafe" ] "fn" identifier
           "(" [ variable { "," variable } [ "," ] ] ")" [ "->" type ] block

enum     -> "enum" identifier "{" variant "," { variant "," } "}"

variant  -> identifier [ "(" type { "," type } ")" ]

block    -> "{" { ( binding | expression [ ";" ] ) } "}"
```

## Statement

```
binding  -> "let" variable "=" expression ";"

variable -> pattern ":" type

type     -> "u8"
          | "usize"
          | "bool"
          | "char"
          | identifier
          | ( "&" [ "mut" ] | "*" "mut" ) type
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
            | call
            | "(" expression ")"
            | [ "unsafe" ] block
            | if
            | while
            | match )
```

## Remaining Control Flow

```
if      -> "if" expression block [ "else" [ if | block ] ]

while   -> "while" expression block

match   -> "match" expression "{" { arm } "}"

arm     -> pattern "=>" expression ","

pattern -> literal
         | [ "mut" ] identifier
         | identifier "::" identifier [ "(" pattern { "," pattern } [ "," ] ")" ] )
         | "_"

call    -> path "(" [ expression { "," expression } [ "," ] ] ")"

path    -> identifier { "::" identifier }
```

## Literals

```
literal    -> integer | string | character | boolean

integer    -> digit { digit }

string     -> """ { printable_character } """

character  -> "'" printable_character "'"

boolean    -> "true" | "false"

digit      -> "0" | ... | "9"

identifier -> ( letter | "_" ) { letter | digit | "_" }

letter     -> "a" | ... | "z" | "A" | ... | "Z"
```

