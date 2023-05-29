//use std::io;
use std::env;
use std::process::exit;
use quick_xml::reader::Reader;
//use virt::connect::Connect;

fn check_args() {

    let argv: Vec<String> = env::args().collect();

    if argv.len() != 2 {
        eprintln!("FATAL ERROR: INCORRECT USAGE. \
Use with one argument as absolute path to domain xml: \
virsh-persist </absolute/path/to/domain/xml.xml>");
        exit(exitcode::USAGE)
    }
}

fn main() {

    check_args();

}
