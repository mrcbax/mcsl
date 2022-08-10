pub static INSERT_SERVER: &str = "INSERT INTO servers (ip, port, version, version_pretty, max_players, online_players, motd, favicon, last_checked) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)";
pub static INSERT_PLAYER: &str = "INSERT INTO players (mojang_uuid, username, last_seen, latest_server) VALUES (?1,?2,?3,?4)";
pub static INSERT_PLAYER_HISTORY: &str = "INSERT INTO player_history (player, server, seen_on) VALUES (?1,?2,?3)";
