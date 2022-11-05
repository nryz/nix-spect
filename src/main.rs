use clap::{arg, command};
use std::process::Command;

fn get_completions(path: String) -> Result<Vec<String>, String> {
    let result = Command::new("nix")
        .arg("eval")
        .arg("--raw")
        .arg(&path)
        .env("NIX_GET_COMPLETIONS", "3")
        .output()
        .expect("command failed to run");

    if !result.status.success() {
        let error = String::from_utf8(result.stderr).unwrap();
        return Err(format!("Error: {}", error));
    }

    let out = String::from_utf8(result.stdout).unwrap();

    let mut completions: Vec<String> = Vec::new();
    for s in out.lines() {
        if !s.eq("attrs") {
            completions.push(s.to_string().replace(&path, ""));
        }
    }
    Ok(completions)
}

fn main() {
    let matches = command!()
        .arg(arg!([flake] "flake path").required(true))
        .get_matches();

    let flake_path = matches
        .get_one::<String>("flake")
        .expect("expected a valid flake path");

    let mut path: String = flake_path.to_owned();

    if !path.contains('#') {
        path.push('#');
    }

    if !path.ends_with('.') && !path.ends_with('#') {
        path.push('.');
    }

    println!("whole_path: {}", path);

    let completions = get_completions(path).expect("Error while getting completions");

    for c in completions {
        println!("completion: {}", c);
    }
}
