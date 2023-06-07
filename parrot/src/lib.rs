mod request;

use std::collections::HashMap;
use std::io::Result;

use serde::Deserialize;

use crate::request::request_streaming;

use self::request::request_one;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GameMeta {
    game_id: String,
}

#[derive(Deserialize, Debug)]
struct GameEvent {
    fen: String,
}

/// Gets the IDs of the current Lichess TV games.
pub fn tv_games() -> Result<HashMap<String, String>> {
    request_one("https://lichess.org/api/tv/channels").map(|games: HashMap<String, GameMeta>| {
        games
            .into_iter()
            .map(|(kind, meta)| (kind, meta.game_id))
            .collect()
    })
}

/// Watches a Lichess game, returning the FEN as it changes.
pub fn watch_game(id: impl AsRef<str>) -> Result<impl Iterator<Item = Result<String>>> {
    let id = id.as_ref();
    request_streaming(&format!("https://lichess.org/api/stream/game/{id}"))
        .map(|events| events.map(|event| event.map(|event: GameEvent| event.fen)))
}
