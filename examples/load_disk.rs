use std::env;
use std::process;

use badger_rs::{Database, IteratorOptions, OpenOptions};

fn main() {
    if let Err(err) = run() {
        eprintln!("load_disk failed: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("/tmp/badger-rs-example"));

    let db = Database::open(
        &path,
        &OpenOptions {
            value_dir: path.clone(),
            ..OpenOptions::default()
        },
    )?;

    let reader = db.new_txn(true)?;
    let mut iter = reader.iterator(&IteratorOptions {
        prefix: Some(b"cross/".to_vec()),
        ..IteratorOptions::default()
    })?;

    println!("loaded database at {path}");
    while iter.has_next()? {
        let key = String::from_utf8(iter.key()?)?;
        match iter.value()? {
            Some(value) => println!("{key}={}", String::from_utf8(value)?),
            None => println!("{key}=<nil>"),
        }
    }

    Ok(())
}
