use std::fs;
use std::env;
use std::path::Path;
use std::time::Duration;
use std::process::exit;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use virt::connect::Connect;
use virt::domain::Domain;
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use crossbeam::channel::{select, self, Sender, after};

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

fn get_domain_name(file_path: &String) -> Vec<String> {

    let xml                       = fs::read_to_string(file_path).unwrap();
    let mut reader                = Reader::from_str(&xml);
    let mut buf                   = Vec::new();
    let mut domain                = String::new();
    let mut dom_info: Vec<String> = Vec::new();

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
    dom_info.push(domain);
    dom_info.push(xml);
    dom_info

}

fn define_domain(conn: &Connect, dom_info: &Vec<String>) -> Domain {

    let dom = match Domain::lookup_by_name(conn, &dom_info[0]) {
        Ok(d) => d,
        _ => {
            let dom = match Domain::define_xml(conn, &dom_info[1]) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("FATAL ERROR: CANNOT DEFINE DOMAIN\n\
{}", e);
                    exit(exitcode::UNAVAILABLE)
                }
            };
            dom
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

fn cleanup(mut conn: Connect, mut dom: Domain) {
    
    let timeout = after(Duration::from_secs(10));
    Domain::shutdown(&dom);
    
    loop {
        select! {
            recv(timeout) -> _ => {
                Domain::destroy(&dom);
            },
            default => {
                match Domain::get_state(&dom) {
                    Ok((4,_)) => {
                        break;
                    }
                    Ok((5,_)) => {
                        break;
                    },
                    _ => {}
                }
            }
        }
    }
    dom.free();
    conn.close();
    
}

fn await_interrupt(signal_channel: Sender<()>) {

    let mut signals = Signals::new(&[
        SIGTERM,
        SIGINT,
        SIGABRT,
        SIGQUIT,
    ]).unwrap();

    for _s in &mut signals {
        signal_channel.send(());
    }

}

fn main() {

    let (send_int, recv_int) = channel::unbounded();
    std::thread::spawn(move || { await_interrupt(send_int)});

    let argv: Vec<String> = env::args().collect();

    check_args(&argv);

    let xml_path = &argv[1];
    let dom_info = get_domain_name(xml_path);

    let conn = match Connect::open("qemu:///system") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FATAL ERROR: COULD NOT CONNECT TO QEMU HYPERVISOR\n\
{}", e);
            exit(exitcode::UNAVAILABLE)
        }
    };

    let mut dom = define_domain(&conn, &dom_info);

    loop {
        select! {
            recv(recv_int) -> _ => {
                cleanup(conn, dom);
                break;
            },
            default => {
                dom = define_domain(&conn, &dom_info);
                start_domain(&dom);
            }
        }
    }

}
