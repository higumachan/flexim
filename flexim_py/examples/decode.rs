use polars::prelude::{IpcReader, SerReader};
use std::io::{BufReader, Cursor, Read};
use std::process::exit;

fn main() {
    let mut reader = BufReader::new(std::io::stdin());

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).unwrap();

    let cursor = Cursor::new(buf);
    let reader = IpcReader::new(cursor);

    if let Ok(df) = reader.finish() {
        println!("{}", df);
        println!("{:?}", &df.schema());
    } else {
        exit(-1);
    }
}
