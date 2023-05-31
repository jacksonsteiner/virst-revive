//use std::io;
use std::env;
use std::path::Path;
use std::process::exit;
//use quick_xml::reader::Reader;
//use virt::connect::Connect;

fn check_args(argv: &Vec<String>) {

    if argv.len() != 2 {
        eprintln!("FATAL ERROR: INCORRECT USAGE.\n\
Use with one argument as absolute path to domain xml: \
virsh-persist </absolute/path/to/domain/xml.xml>");
        exit(exitcode::USAGE)
    }

    let file_path = &argv[1];

    match Path::new(file_path).try_exists() {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("FATAL ERROR: FILE NOT FOUND\n\
The given file could not be found. \
If it truly does exist, make sure the user this program is being run as \
has read permissions to the file. If on a machine with SELINUX enabled, \
check for possible policy denials.");
            exit(exitcode::NOINPUT)
        }
        Err(e) => {
            eprintln!("{}", e);
            exit(exitcode::NOINPUT)
        }
    }
}

fn main() {

    let argv: Vec<String> = env::args().collect();

    check_args(&argv);

}
