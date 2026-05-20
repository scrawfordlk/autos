fn main() -> usize {
    if false {
        return 1; // type: !
    } else
        // type: usize
        if true {
            if false {
                3
            } else
                if false {
                    return 2;
                } else {
                    42
                }
        } else {
            return 3;
        }
}
