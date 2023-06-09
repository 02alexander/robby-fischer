use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use anyhow::Result;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Deserializer;

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
    in_runtime(async {
        let games = reqwest::get("https://lichess.org/api/tv/channels")
            .await?
            .json::<HashMap<String, GameMeta>>()
            .await?;

        Ok(games
            .into_iter()
            .map(|(kind, meta)| (kind, meta.game_id))
            .collect())
    })
}

/// Watches a Lichess game, returning the FEN as it changes.
pub fn watch_game(id: impl AsRef<str>, mut on_fen: impl FnMut(&str)) -> Result<()> {
    let url = format!("https://lichess.org/api/stream/game/{}", id.as_ref());
    let (send_event, recv_event) = std::sync::mpsc::channel::<Result<GameEvent>>();

    std::thread::spawn(move || {
        in_runtime(async {
            let response = match reqwest::get(url).await {
                Ok(response) => response,
                Err(err) => {
                    _ = send_event.send(Err(err.into()));
                    return;
                }
            };
            let mut stream = response.bytes_stream();
            let mut buffer = Vec::new();
            while let Some(result) = stream.next().await {
                match result {
                    Ok(bytes) => {
                        buffer.extend_from_slice(&bytes);

                        let mut deser = Deserializer::from_slice(&buffer).into_iter();
                        loop {
                            match deser.next() {
                                Some(Ok(event)) => {
                                    if send_event.send(Ok(event)).is_err() {
                                        return;
                                    }
                                }
                                Some(Err(err)) if err.is_eof() => {
                                    buffer.drain(..deser.byte_offset());
                                    break;
                                }
                                Some(Err(err)) => {
                                    _ = send_event.send(Err(err.into()));
                                    return;
                                }
                                None => {
                                    buffer.clear();
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        _ = send_event.send(Err(err.into()));
                        return;
                    }
                }
            }
        })
    });

    std::thread::sleep(Duration::from_millis(500));

    while let Ok(result) = recv_event.recv() {
        let mut event = result?;
        while let Ok(result) = recv_event.try_recv() {
            event = result?;
        }

        on_fen(&event.fen);
    }

    Ok(())
}

fn in_runtime<R>(f: impl Future<Output = R>) -> R {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap()
        .block_on(f)
}
