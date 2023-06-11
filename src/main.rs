//use std::io;
use std::fs;
use std::env;
use std::path::Path;
use std::process::exit;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
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

fn get_domain_name(file_path: &String) -> String {

    let xml        = fs::read_to_string(file_path).unwrap();
    let mut reader = Reader::from_str(&xml);
    let mut buf    = Vec::new();
    let mut domain = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"name" => {
                    domain = reader.read_text(e.name()).unwrap().to_string();
                    break;
                }
                _ => {},
            },
            Ok(Event::Eof) => unreachable!(),
            _ => {},
        }
        buf.clear();
    }

    domain

}

fn main() {

    let argv: Vec<String> = env::args().collect();

    check_args(&argv);

    let domain = get_domain_name(&argv[1]);
    println!("{}", domain);

}
