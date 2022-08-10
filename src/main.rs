use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Duration;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};


use craftping::tokio::ping;
use rusqlite::params;
use tokio::time::timeout;
//use tokio::sync::oneshot;
use tokio::net::TcpStream;

pub mod types;

async fn do_ping(conn: &tokio_rusqlite::Connection, hostname: String, port: u16) {

    match timeout(Duration::from_millis(1500), {
        TcpStream::connect((hostname.clone(), port))
    }).await {
        Ok(result) => {
            match result {
                Ok(mut stream) => {
                    match timeout(Duration::from_millis(1500), {
                        ping(&mut stream, hostname.as_str(), port)
                    }).await {
                        Ok(result) => {
                            match result {
                                Ok(response) => {
                                    println!("{}\thit", hostname);
                                    let mut description = response.description.text;
                                    for item in response.description.extra {
                                        description.push_str(item.text.as_str());
                                    }
                                    description = description.replace("'\n", "\\n").replace("\r\n", "\\n").replace("\"", "&quot;").replace("'", "&apos;").replace(",", "&comma;");
                                    let favicon = match response.favicon {
                                        Some(favicon) => base64::encode(favicon),
                                        None => "".into()
                                    };
                                    let version_pretty = response.version.replace("'\n", "\\n").replace("\r\n", "\\n").replace("\"", "&quot;").replace("'", "&apos;").replace(",", "&comma;");
                                    let time = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs();
                                    conn.call(move |conn|{
                                        match conn.execute(
                                            types::INSERT_SERVER,
                                            params![hostname.clone(), port.clone(), response.protocol.clone(), version_pretty.clone(), response.max_players.clone(), response.online_players.clone(), description.clone(), favicon.clone(), time.clone()]
                                        ) {
                                            Ok(_) => {
                                                match response.sample {
                                                    Some(s) => {
                                                        let server_id = conn.last_insert_rowid();
                                                        for player in s {
                                                            match conn.execute(
                                                                types::INSERT_PLAYER,
                                                                params![player.id, player.name, time.clone(), server_id]
                                                            ) {
                                                                Ok(_) => {
                                                                    let player_id = conn.last_insert_rowid();
                                                                    match conn.execute(
                                                                        types::INSERT_PLAYER_HISTORY,
                                                                        params![player_id, server_id, time.clone()]
                                                                    ) {
                                                                        Ok(_) => (),
                                                                        Err(e) => panic!("failed to execute INSERT_PLAYER_HISTORY {}", e)
                                                                    }
                                                                },
                                                                Err(_) => ()
                                                            }
                                                        }
                                                    },
                                                    None => ()
                                                }
                                            },
                                            Err(_) => ()
                                        }
                                    }).await;
                                    //return Some(format!("{},{},\"{}\",{},{},{},\"{}\",{},\"{}\"", hostname, port, response.version.replace("'\n", "\\n").replace("\r\n", "\\n").replace("\"", "&quot;").replace("'", "&apos;").replace(",", "&comma;"), response.protocol, response.max_players, response.online_players, description.as_str(), time, favicon));
                                },
                                Err(_) => println!("{}\tmiss", hostname)
                            }
                        },
                        Err(_) => println!("{}\tmiss", hostname)
                    }
                },
                Err(_) => println!("{}\tmiss", hostname)
            }
        },
        Err(_) => println!("{}\ttimeout", hostname)
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let db: Arc<Mutex<VecDeque<(String, u16)>>> = Arc::new(Mutex::new(VecDeque::new()));
    let conn = tokio_rusqlite::Connection::open("./data/servers.sqlite").await.expect("failed to open database");
    conn.call(|conn| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS servers (
	               id                INTEGER PRIMARY KEY AUTOINCREMENT,
                 ip                TEXT NOT NULL UNIQUE,
                 port              INTEGER NOT NULL,
                 version           INTEGER,
                 version_pretty    TEXT,
                 max_players       INTEGER,
                 online_players    INTEGER,
                 motd              TEXT,
                 favicon           TEXT,
                 last_checked      INTEGER NOT NULL
             );", []).expect("failed to create table 'servers'");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS players (
                 id                INTEGER PRIMARY KEY AUTOINCREMENT,
                 mojang_uuid       TEXT UNIQUE,
                 username          TEXT NOT NULL,
                 last_seen         INTEGER NOT NULL,
	               latest_server     INTEGER REFERENCES servers (id)
             );", []).expect("failed to create table 'players'");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS player_history (
                 player            INTEGER REFERENCES players (id),
                 server            INTEGER REFERENCES servers (id),
                 seen_on           INTEGER NOT NULL
             );", []).expect("failed to create table 'player_history'");
    }).await;
    let mut line_num = 0;
    let file = File::open("data/found_formatted.txt").unwrap();
    let reader = BufReader::new(file);

    for (num, line) in reader.lines().enumerate() {
        if num >= line_num {
            line_num = num;
            let line = line.unwrap();
            let address_parts: Vec<&str> = line.split(':').collect();
            let hostname = address_parts[0];
            let port = address_parts[1].parse::<u16>().unwrap();

            db.lock().unwrap().push_back((hostname.into(), port));
        }
    }
    let mut task_workers: Vec<(bool, tokio::task::JoinHandle<Option<bool>>)> = vec!();
    loop {
        if task_workers.len() < 150 {
            let db = db.clone();
            let conn = conn.clone();
            task_workers.push((false, tokio::spawn(async move {
                while !db.lock().unwrap().is_empty() {
                    let (hostname, port) = db.lock().unwrap().pop_front().unwrap();
                    do_ping(&conn, hostname.into(), port).await;
                }
                return Some(true);
            })));
        } else {
            let mut num_removed = 0;
            let mut num_finished = 0;
            for i in 0..task_workers.len() {
                let j = i - num_removed;
                if j < task_workers.len() {
                    if !task_workers[j].0 && task_workers[j].1.is_finished() {
                        let result = task_workers.remove(j).1.await;
                        match result {
                            Ok(o) => {
                                if o.is_some() {
                                    task_workers.push((true, tokio::spawn(async move {
                                        return Some(true);
                                    })));
                                } else {
                                    num_removed = num_removed + 1;
                                }
                            },
                            Err(_) => num_removed = num_removed + 1
                        }
                    }
                    if task_workers[j].0 {
                        num_finished = num_finished + 1;
                    }
                } else {
                    break;
                }
            }
            if num_finished >= 150 {
                std::process::exit(0);
            }
            if db.lock().unwrap().is_empty() {
                std::process::exit(0)
            }
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }
}
