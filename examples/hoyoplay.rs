fn main() {
    match game_scanner::hoyoplay::executable() {
        Ok(path) => println!("Launcher: {}", path.display()),
        Err(error) => eprintln!("{error}"),
    }
    match game_scanner::hoyoplay::games() {
        Ok(games) => println!("{}", serde_json::to_string_pretty(&games).unwrap()),
        Err(error) => eprintln!("{error}"),
    }
}
