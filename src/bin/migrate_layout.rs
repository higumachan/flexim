use clap::Parser;
use serde_json::Value as JsonValue;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long)]
    input_layout_path: PathBuf,
    #[clap(short, long)]
    output_layout_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    let mut reader = BufReader::new(File::open(args.input_layout_path).unwrap());
    let mut json_value: JsonValue = serde_json::from_reader(&mut reader).unwrap();

    for layout in json_value
        .as_array_mut()
        .expect("レイアウトファイルのルートはリストになっている")
    {
        let tree = layout.get_mut("tree").expect("treeがない");

        let tree = tree.as_object_mut().unwrap();
        tree.entry("width".to_string()).or_insert(JsonValue::Null);
        tree.entry("height".to_string()).or_insert(JsonValue::Null);
    }

    let output_layout_file = File::create(args.output_layout_path).unwrap();
    serde_json::to_writer_pretty(output_layout_file, &json_value).unwrap();
}
