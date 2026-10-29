//! Windows: cargo run --example wegame
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("WeGame: {:?}", game_scanner::wegame::executable());
    let games = game_scanner::wegame::games()?;
    println!("{}", serde_json::to_string_pretty(&games)?);
    for game in games {
        let found = game_scanner::wegame::find(&game.id)?;
        assert_eq!(found.path, game.path);
    }
    Ok(())
}
