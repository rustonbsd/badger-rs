use std::env;
use std::process;

use badger_rs::{Database, IteratorOptions, OpenOptions, Txn};

fn main() {
    if let Err(err) = run() {
        eprintln!("iter_bounds_reverse failed: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("/tmp/badger-iter-example"));

    let db = Database::open(
        &path,
        &OpenOptions {
            value_dir: path.clone(),
            ..OpenOptions::default()
        },
    )?;

    db.drop_all()?;
    seed(&db)?;

    let reader = db.new_txn(true)?;
    println!("iterating database at {path}");
    print_range(
        &reader,
        "forward range [cross/015, cross/045)",
        IteratorOptions {
            start: Some(b"cross/015".to_vec()),
            end: Some(b"cross/045".to_vec()),
            ..IteratorOptions::default()
        },
    )?;
    print_range(
        &reader,
        "reverse range [cross/015, cross/045)",
        IteratorOptions {
            start: Some(b"cross/015".to_vec()),
            end: Some(b"cross/045".to_vec()),
            reverse: true,
            ..IteratorOptions::default()
        },
    )?;

    Ok(())
}

fn seed(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let writer = db.new_txn(false)?;
    writer.set(b"cross/010", Some(b"ten"))?;
    writer.set(b"cross/020", Some(b"twenty"))?;
    writer.set(b"cross/030", Some(b"thirty"))?;
    writer.set(b"cross/040", Some(b"forty"))?;
    writer.set(b"cross/050", Some(b"fifty"))?;
    writer.commit()?;
    Ok(())
}

fn print_range(
    reader: &Txn,
    label: &str,
    options: IteratorOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut iter = reader.iterator(&options)?;
    println!("{label}");
    while iter.has_next()? {
        let key = String::from_utf8(iter.key()?)?;
        let value = iter.value()?.expect("seeded values are non-nil");
        println!("{key}={}", String::from_utf8(value)?);
    }
    Ok(())
}
