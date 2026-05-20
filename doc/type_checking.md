# Type system

## Types

- `usize`
- `u8`
- `bool`
- `char`
- `&str`
- `&T`
- `&mut T`
- `*mut T`

- `!`
- `()` (implicitly)

## Type Checking

In general the following rule applies: For two values of type $t_1$ and $t_2$ to be compatible, it must hold that $t_1 == t_2$. That is, typical strongly-typed language.

- On top of this the following holds:
  - If $t_1 := !$, then $t_1 == t_2$ for any $t_2 \in T$.
  - Let $e$ be an expression made up of multiple sub-expressions of type $t_1, t_2, \dots$.
    1. From the general rule and the rule above, it holds that $t_1 == t_2$ for any pair of sub-expressions.
    2. For two sub-expressions $t_1$ and $t_2$, the type $t = coalesce(t_1, t_2)$ of the expression $e$ is:
       1. If neither are of type $!$: $t = t_1 = t_2$
       2. $(t_1 = ! \Rightarrow t = t_2) \lor (t_2 = ! \Rightarrow t = t_1)$
       - e.g.:

         ```rust
         let x: ! = return;
         let y: u8 = 0;
         let composite: u8 = if cond { return } else { 0 }
         let composite: u8 = if cond { 0 } else { return }
         ```
