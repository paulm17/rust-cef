use toml;
use std::fs;

fn main() {
    let content = fs::read_to_string("config.toml").unwrap();
    println!("File content:\n{}", content);
    let v: toml::Value = toml::from_str(&content).unwrap();
    println!("Parsed: {:#?}", v);
}
