enum Generic<T> {
    Gen(T),
}

enum Variable {
    Variable(usize, bool),
}

enum RefVar<'a> {
    Variable(&'a Variable),
}

fn test_field_enum_reference() -> usize {
    let var: Variable = Variable::Variable(21, true);
    let RefVar::Variable(Variable::Variable(int, b)): RefVar = RefVar::Variable(&var);
    if *b { *int } else { 0 }
}

fn test_reference_to_generic() -> usize {
    let var: Variable = Variable::Variable(15, true);
    let Generic::Gen(Variable::Variable(int, b)): &Generic<Variable> = &Generic::<Variable>::Gen(var);
    if *b { *int } else { 0 }
}

fn test_generic_reference() -> usize {
    let var: Variable = Variable::Variable(6, true);
    let Generic::Gen(Variable::Variable(int, b)): Generic<&Variable> = Generic::<&Variable>::Gen(&var);
    if *b { *int } else { 0 }
}

fn main() {
    unsafe { exit(test_field_enum_reference() + test_reference_to_generic() + test_generic_reference()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
