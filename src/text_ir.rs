use crate::ast::IntoTextIR;
use crate::sysy;
use std::fs::{read_to_string, File};
use std::io::{Result, Write};

pub fn generate_to_string(input_path: &str) -> String {
    let input = read_to_string(input_path).unwrap();
    let ast = sysy::CompUnitParser::new().parse(&input).unwrap();
    ast.into_text_ir()
}

pub fn generate_to_file(input_path: &str, output_path: &str) -> Result<()> {
    let ir = generate_to_string(input_path);
    let mut f = File::create(output_path)?;
    f.write_all(ir.as_bytes())?;

    Ok(())
}
