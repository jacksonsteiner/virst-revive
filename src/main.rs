use std::fs;
use std::env;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::process::exit;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use virt::connect::Connect;
use virt::domain::Domain;

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
                    domain.push_str(&reader.read_text(e.name()).unwrap().to_string());
                    break;
                }
                _ => {},
            },
            Ok(Event::Eof) => {
                eprintln!("FATAL ERROR: INVALID DOMAIN XML\n\
The 'name' tag in the provided domain xml could not be found.");
                exit(exitcode::NOINPUT)
            },
            _ => {},
        }
        buf.clear();
    }
    domain

}

fn define_domain(conn: &Connect, domain: &String, file_path: &String) -> Domain {

    let dom = match Domain::lookup_by_name(conn, domain) {
        Ok(d) => d,
        _ => {
            match Domain::save_image_define_xml(conn, file_path,) {
                Ok(d) => {
                    dom = Domain::lookup_by_name(conn, domain);
                }
                Err(e) => {
                    eprintln!("FATAL ERROR: CANNOT DEFINE DOMAIN\n\
{}", e);
                    exit(exitcode::UNAVAILABLE)
                }
            };
        }
    };
    dom

}

fn start_domain(dom: &Domain) {

    match Domain::get_state(dom) {
        Ok((0,_)) => {
            match Domain::create(dom) {
                Err(e) => {
                    eprintln!("FATAL ERROR: CANNOT START DOMAIN\n\
{}", e);
                    exit(exitcode::UNAVAILABLE)
                }
                _ => {}
            }
        }
        Ok((1,_)) => {},
        Ok((2,_)) => {
            match Domain::resume(dom) {
                Err(e) => {
                    eprintln!("FATAL ERROR: COULD NOT RESUME DOAMIN FROM BLOCKED STATE\n\
{}", e);
                    exit(exitcode::UNAVAILABLE)
                }
                _ => {}
            }
        }
        Ok((3,_)) => {
            match Domain::resume(dom) {
                Err(e) => {
                    eprintln!("FATAL ERROR: COULD NOT RESUME DOAMIN FROM PAUSED STATE\n\
{}", e);
                    exit(exitcode::UNAVAILABLE)
                }
                _ => {}
            }
        }
        Ok((4,_)) => {},
        Ok((5,_)) => {
            match Domain::create(dom) {
                Err(e) => {
                    eprintln!("FATAL ERROR: CANNOT START DOMAIN\n\
{}", e);
                    exit(exitcode::UNAVAILABLE)
                }
                _ => {}
            }
        }
        Ok((6,_)) => {
            match Domain::create(dom) {
                Err(e) => {
                    eprintln!("FATAL ERROR: CANNOT START DOMAIN FROM CRASH\n\
{}", e);
                    exit(exitcode::UNAVAILABLE)
                }
                _ => {}
            }
        }
        Ok((7,_)) => {
            match Domain::create(dom) {
                Err(e) => {
                    eprintln!("FATAL ERROR: CANNOT START DOMAIN FROM PMSUSPENSION\n\
{}", e);
                    exit(exitcode::UNAVAILABLE)
                }
                _ => {}
            }
        }
        Err(e) => {
            eprintln!("FATAL ERROR: ERROR RETRIEVING DOMAIN STATE\n\
{}", e);
            exit(exitcode::UNAVAILABLE)
        }
        _ => {
            eprintln!("FATAL ERROR: UNKOWN DOMAIN STATE");
            exit(exitcode::UNAVAILABLE)
        }
    }
}

fn main() {

    let argv: Vec<String> = env::args().collect();

    check_args(&argv);

    let xml_path = &argv[1];
    let domain = get_domain_name(xml_path);

    let conn = match Connect::open("qemu:///system") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FATAL ERROR: COULD NOT CONNECT TO QEMU HYPERVISOR\n\
{}", e);
            exit(exitcode::UNAVAILABLE)
        }
    };

    let duration = Duration::from_secs(5);
    
    loop {
        let dom = define_domain(&conn, &domain, xml_path);
        start_domain(&dom);
        thread::sleep(duration);
    }

}
