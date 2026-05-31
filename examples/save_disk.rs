use std::env;
use std::process;

use badger_rs::{Database, OpenOptions};

fn main() {
    if let Err(err) = run() {
        eprintln!("save_disk failed: {err}");
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

    let writer = db.new_txn(false)?;
    writer.set(b"cross/alpha", Some(b"one"))?;
    writer.set(b"cross/beta", Some(b"two"))?;
    writer.set(b"cross/empty", None)?;
    writer.commit()?;

    println!("saved database at {path}");
    println!("wrote cross/alpha=one");
    println!("wrote cross/beta=two");
    println!("wrote cross/empty=<nil>");

    Ok(())
}
