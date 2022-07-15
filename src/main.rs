use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use craftping::sync::ping;

fn do_ping(hostname: String, port: u16) -> Option<String> {
    let thread = std::thread::spawn(move || {
        match TcpStream::connect_timeout(&std::net::SocketAddr::new(hostname.parse().unwrap(), port), std::time::Duration::new(2, 0)) {
            Ok(mut stream) => {
                match ping(&mut stream, hostname.as_str(), port) {
                    Ok(pong) => {
                        println!("{}:{} hit", hostname, port);
                        return Some(format!("{},{},{},{},{},{},{}", hostname, port, enquote::enquote('\"', pong.version.as_str()), pong.protocol, pong.max_players, pong.online_players, enquote::enquote('\"', pong.description.text.as_str())));
                    },
                    Err(_) => println!("{}:{} miss", hostname, port)
                }
            },
            Err(_) => println!("{}:{} miss", hostname, port)
        }
        return None;
    });
    let mut counter = 5000;
    while counter > 0 {
        if thread.is_finished() {
            match thread.join() {
                Ok(o) => return o,
                Err(_) => return None
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
        counter = counter - 1;
    }
    return None;
}

fn main() {
    let mut line_num = 0;
    let file = File::open("data/found_formatted.txt").unwrap();
    let reader = BufReader::new(file);

    let mut outfile = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("data/servers.txt")
        .unwrap();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    if std::path::Path::new("data/mcsl.resume").exists() {
        let resumefromfile = File::open("data/mcsl.resume").unwrap();
        let mut resumereader = BufReader::new(resumefromfile);
        let mut first_line = String::new();
        let _ = resumereader.read_line(&mut first_line);
        line_num = first_line.parse().unwrap();
    }
    for (num, line) in reader.lines().enumerate() {
        if running.load(Ordering::SeqCst) {
            if num >= line_num {
                line_num = num;
                let line = line.unwrap();
                //std::thread::sleep(std::time::Duration::from_secs(1));
                let address_parts: Vec<&str> = line.split(':').collect();
                let hostname = address_parts[0];
                let port = address_parts[1].parse::<u16>().unwrap();
                match do_ping(hostname.to_string(), port) {
                    Some(s) => writeln!(outfile, "{}", s).unwrap(),
                    None => ()
                }
            }
        } else {
            let mut resumefile = OpenOptions::new()
                .write(true)
                .append(false)
                .create(true)
                .open("data/mcsl.resume")
                .unwrap();
            writeln!(resumefile, "{}", line_num).unwrap();
        }
    }
}
