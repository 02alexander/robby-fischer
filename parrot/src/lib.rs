use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Deserializer;
use std::sync::mpsc::{channel, Receiver};
use tokio::select;
use tokio::time::interval;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GameMeta {
    game_id: String,
}

#[derive(Deserialize, Debug)]
struct GameEvent {
    fen: String,
}

#[derive(Deserialize)]
#[serde(tag = "t", content = "d", rename_all = "lowercase")]
enum StalkEvent {
    Move { fen: String },
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

pub fn stalk_game(id: impl AsRef<str>) -> Result<Receiver<Result<String>>> {
    let sri = "9uFl575FsPGH"; // Magic string, must not be changed.
    let url = format!(
        "wss://socket5.lichess.org/watch/{}/white/v6?sri={}&v=0:8765",
        id.as_ref(),
        sri
    );
    let (send_event, recv_event) = channel::<Result<String>>();

    std::thread::spawn(move || {
        in_runtime(async {
            let (ws, _) = match connect_async(url).await {
                Ok(stuff) => stuff,
                Err(err) => {
                    _ = send_event.send(Err(err.into()));
                    return;
                }
            };
            let (mut writer, mut reader) = ws.split();
            let mut interval = interval(Duration::from_millis(3500));
            loop {
                select! {
                    read = reader.next() => {
                        match read {
                            Some(Ok(Message::Text(s))) if s!= "0" => {
                                match serde_json::from_str::<StalkEvent>(&s) {
                                    Ok(StalkEvent::Move{ fen }) => {
                                        if send_event.send(Ok(fen)).is_err() {
                                            return;
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("{}", &s);
                                        eprintln!("{:?}", e);
                                    }
                                }
                            },
                            Some(Ok(_)) => {}
                            Some(Err(e)) => {
                                let _ = send_event.send(Err(e.into()));
                            }
                            _ => { break },
                        }
                    },
                    _ = interval.tick() => {
                        if let Err(err) = writer.send(Message::Text("null".into())).await {
                            _ = send_event.send(Err(err.into()));
                        }
                    }
                }
            }
        })
    });
    Ok(recv_event)
}

/// Watches a Lichess game, returning the FEN as it changes.
pub fn watch_game(id: impl AsRef<str>) -> Result<Receiver<Result<String>>> {
    let url = format!("https://lichess.org/api/stream/game/{}", id.as_ref());
    let (send_event, recv_event) = channel::<Result<String>>();

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

                        let mut deser = Deserializer::from_slice(&buffer).into_iter::<GameEvent>();
                        loop {
                            match deser.next() {
                                Some(Ok(event)) => {
                                    if send_event.send(Ok(event.fen)).is_err() {
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

    Ok(recv_event)
}

fn in_runtime<R>(f: impl Future<Output = R>) -> R {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap()
        .block_on(f)
}
