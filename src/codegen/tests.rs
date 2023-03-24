#![cfg(test)]

use std::{fs::remove_file, io::Write};

use super::*;

const TMP_IPATH: &str = "itmp.c";
const TMP_OPATH: &str = "otmp.S";

#[test]
fn dangling_exp() {
    let source_code = "
    int main() {
        int a = 1;
        a + 1;
        return 0;
    }";
    prepare(source_code);
    assert!(generate_code(TMP_IPATH, TMP_OPATH).is_ok());
    clean_up();
}

#[test]
fn if_else_return() {
    let source_code = "
    int main() {
        int a = 10;
        if (a > 4)
            return a;
        else
            return a / 2;
    }";
    prepare(source_code);
    assert!(generate_code(TMP_IPATH, TMP_OPATH).is_ok());
    clean_up();
}

#[test]
fn if_else_return2() {
    let source_code = "
    int main() {
        int a = 10;
        if (a > 4)
            return a;
        else if (a < 5)
            return a / 2;
        else
            return a;
    }";
    prepare(source_code);
    assert!(generate_code(TMP_IPATH, TMP_OPATH).is_ok());
    clean_up();
}

fn prepare(source_code: &str) {
    write_into(TMP_IPATH, source_code);
}

fn clean_up() {
    remove_file(TMP_IPATH).unwrap();
    remove_file(TMP_OPATH).unwrap();
}

fn write_into(path: &str, content: &str) {
    let mut f = File::create(path).unwrap();
    write!(f, "{content}").unwrap();
}
