use std::fs;
use std::env;
use std::path::Path;
use std::time::Duration;
use std::process;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use virt::connect::Connect;
use virt::domain::Domain;
use log::error;
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use crossbeam::channel::{select, self, Sender, after};

fn check_args(argv: &Vec<String>) {

    if argv.len() != 2 {
        let err = "FATAL ERROR: INCORRECT USAGE.\n\
Use with one argument as absolute path to domain xml: \
virsh-persist </absolute/path/to/domain/xml.xml>";
        eprintln!("{}", err);
        error!("{}", err);
        process::exit(1);
    }

    let file_path = &argv[1];

    match Path::new(file_path).try_exists() {
        Ok(true) => {}
        Ok(false) => {
            let err = "FATAL ERROR: FILE NOT FOUND\n\
The given file could not be found. \
If it truly does exist, make sure the user this program is being run as \
has read permissions to the file. If on a machine with SELINUX enabled, \
check for possible policy denials.";
            eprintln!("{}", err);
            error!("{}", err);
            process::exit(1);
        }
        Err(e) => {
            eprintln!("{}", e);
            error!("{}", e);
            process::exit(1);
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
                let err = "FATAL ERROR: INVALID DOMAIN XML\n\
The 'name' tag in the provided domain xml could not be found.";
                eprintln!("{}", err);
                error!("{}", err);
                process::exit(1);
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
                    let err = "FATAL ERROR: CANNOT DEFINE DOMAIN";
                    eprintln!("{}\n{}", err, e);
                    error!("{}", e);
                    process::exit(1);
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
                    let err = "FATAL ERROR: CANNOT START DOMAIN";
                    eprintln!("{}\n{}", err, e);
                    error!("{}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        Ok((1,_)) => {},
        Ok((2,_)) => {
            match Domain::resume(dom) {
                Err(e) => {
                    let err = "FATAL ERROR: COULD NOT RESUME DOAMIN FROM BLOCKED STATE";
                    eprintln!("{}\n{}", err, e);
                    error!("{}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        Ok((3,_)) => {
            match Domain::resume(dom) {
                Err(e) => {
                    let err = "FATAL ERROR: COULD NOT RESUME DOAMIN FROM PAUSED STATE";
                    eprintln!("{}\n{}", err, e);
                    error!("{}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        Ok((4,_)) => {},
        Ok((5,_)) => {
            match Domain::create(dom) {
                Err(e) => {
                    let err = "FATAL ERROR: CANNOT START DOMAIN";
                    eprintln!("{}\n{}", err, e);
                    error!("{}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        Ok((6,_)) => {
            match Domain::create(dom) {
                Err(e) => {
                    let err = "FATAL ERROR: CANNOT START DOMAIN FROM CRASH";
                    eprintln!("{}\n{}", err, e);
                    error!("{}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        Ok((7,_)) => {
            match Domain::create(dom) {
                Err(e) => {
                    let err = "FATAL ERROR: CANNOT START DOMAIN FROM PMSUSPENSION";
                    eprintln!("{}\n{}", err, e);
                    error!("{}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        Err(e) => {
            let err = "FATAL ERROR: ERROR RETRIEVING DOMAIN STATE";
            eprintln!("{}\n{}", err, e);
            error!("{}", e);
            process::exit(1);
        }
        _ => {
            let err = "FATAL ERROR: UNKOWN DOMAIN STATE";
            eprintln!("{}", err);
            error!("{}", err);
            process::exit(1);
        }
    }

}

fn cleanup(mut conn: Connect, mut dom: Domain) {
    
    let timeout = after(Duration::from_secs(10));
    match Domain::shutdown(&dom) {
        _ => {}
    }
    
    loop {
        select! {
            recv(timeout) -> _ => {
                match Domain::destroy(&dom) {
                    _ => {}
                }
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
    match dom.free() {
        _ => {}
    }
    match conn.close() {
        _ => {}
    }
    
}

fn await_interrupt(signal_channel: Sender<()>) {

    let mut signals = Signals::new(&[
        SIGTERM,
        SIGINT,
        SIGABRT,
        SIGQUIT,
    ]).unwrap();

    for _s in &mut signals {
        match signal_channel.send(()) {
            _ => {}
        }
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
            let err = "FATAL ERROR: COULD NOT CONNECT TO QEMU HYPERVISOR";
            eprintln!("{}\n{}", err, e);
            error!("{}", e);
            process::exit(1);
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
