pub fn lnd(x: i32, y: i32) -> i32 {
    // logical and
    (x != 0 && y != 0) as i32
}

pub fn lor(x: i32, y: i32) -> i32 {
    // logical or
    (x != 0 || y != 0) as i32
}

pub fn is_zero(i: i32) -> i32 {
    (i == 0) as i32
}

pub fn not_zero(i: i32) -> i32 {
    (i != 0) as i32
}

pub fn positive(x: i32) -> i32 {
    x.is_positive() as i32
}

pub fn non_negative(x: i32) -> i32 {
    !x.is_negative() as i32
}
