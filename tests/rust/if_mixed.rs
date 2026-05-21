fn main() -> usize {
    // type: usize
    if false {
        return 1; // type: !
    } else
        // type: usize
        if true {
            // type: usize
            if false {
                return 3;    // type: !
            } else
                if false {
                    return 2; // type: !
                } else {
                    42        // type: usize
                }
        } else {
            return 3; // type: !
        }
}
