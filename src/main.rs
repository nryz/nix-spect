use rnix::types::*;
use std::{env, fs};

fn main() {
    let mut iter = env::args().skip(1).peekable();

    if iter.peek().is_none() {
        eprintln!("Usage: nix-specter <file>");
        return;
    }

    for file in iter {
        let content = match fs::read_to_string(file) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error reading file: {}", err);
                return;
            }
        };

        let ast = rnix::parse(&content);

        for error in ast.errors() {
            println!("Error: {}", error);
        }

        println!("{}", ast.root().dump());
    }
}
