extern crate jellyfish;

use std::env;
use jellyfish::core::fsqual::filesystem_has_good_aio_support;

const TMP_FILE: &'static str = "/tmp/fsqual.tmp";

fn main() {
    std::process::exit(inner_main());
}

fn inner_main() -> i32 {
    let arg = env::args().nth(1);
    let file_path = arg.as_ref().map(|x| x.as_str()).unwrap_or(TMP_FILE);
    match filesystem_has_good_aio_support(file_path) {
        Ok(()) => {
            println!("OK");
            0
        }
        Err(msg) => {
            eprintln!("{}", msg);
            1
        }
    }
}
