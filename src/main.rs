use hl::highlight;
use std::fs;
use std::env;
use std::path::Path;

fn main() {
    let file = env::args().nth(1).unwrap();
    let content = fs::read_to_string(&file).unwrap();
    let file_name = Path::new(&file).file_name().unwrap().to_str().unwrap();

    print!("{}", highlight(&content, file_name, true, true).unwrap());

}
