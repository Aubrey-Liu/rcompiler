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
