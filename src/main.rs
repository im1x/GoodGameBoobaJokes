use std::env;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use dotenv::dotenv;
use futures::{SinkExt, stream::SplitSink, StreamExt};
use rand::seq::SliceRandom;
use rand::thread_rng;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite::protocol::Message, WebSocketStream};
use url::Url;

mod api_client;
mod utils;
mod img;

#[derive(Parser)]
#[command(about, long_about = None)]
struct Cli {
    /// 1 - only save image, 2 - only analyze image
    #[arg(short = 'm', long = "mode", default_value_t = 0)]
    mode: u8,
}

enum Chat {
    Auth,
    Join,
    Say(String),
    Ping,
}

struct Texts {
    pron: Vec<String>,
    anime: Vec<String>,
    urls: Vec<String>,
}

impl Texts {
    fn get_random_string(&self, field: &[String]) -> String {
        let mut rng = thread_rng();
        field.choose(&mut rng).cloned().unwrap_or_default()
    }

    fn get_pron(&self) -> String {
        self.get_random_string(&self.pron)
    }

    fn get_anime(&self) -> String {
        self.get_random_string(&self.anime)
    }

    fn get_url(&self) -> String {
        self.get_random_string(&self.urls)
    }
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    dotenv().ok();

    let ignore_nicks: Vec<String> = env::var("GG_IGNORE_NICKS")
        .expect("GG_IGNORE_NICKS must be set.")
        .split(",")
        .map(str::to_string).collect();

    let texts: Texts = Texts {
        pron: utils::read_file_lines("txt/pron.txt").expect("File not found (txt/pron.txt)"),
        anime: utils::read_file_lines("txt/anime.txt").expect("File not found (txt/anime.txt)"),
        urls: utils::read_file_lines("txt/imgs.txt").expect("File not found (txt/imgs.txt)"),
    };

    let mut urls_list: HashSet<String> = HashSet::new();

    let (ws_stream, response) = connect_async(Url::parse("wss://chat-1.goodgame.ru/chat2/").unwrap()).await.expect("Failed to connect");
    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    let (write, mut read) = ws_stream.split();
    let write = Arc::new(Mutex::new(write));

    gg_chat(&write, Chat::Auth);
    gg_chat(&write, Chat::Join);

    let write_clone = Arc::clone(&write);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(23)).await;
            println!("============== PING");
            gg_chat(&write_clone, Chat::Ping);
        }
    });

    loop {
        if let Some(Ok(message)) = read.next().await {
            let msg = match message {
                Message::Text(s) => s,
                _ => {
                    eprintln!("Error reading message");
                    continue;
                }
            };

            let parsed: serde_json::Value = serde_json::from_str(&msg).expect("Can't parse to JSON");

            if parsed["type"] == "message"{
                let url = utils::get_urls(&parsed["data"]["text"].to_string());
                let nick = parsed["data"]["user_name"].to_string().trim_matches('"').to_owned();
                if urls_list.contains(&url) || ignore_nicks.contains(&nick) {
                    continue;
                }
                urls_list.insert(url.clone());
                let ext = utils::get_ext_from_url(&url);
                if !ext.is_empty() {
                    let mut content = match img::get_image(&url).await {
                        Ok(content) => content,
                        Err(error) => {
                            eprintln!("Error getting image: {}", error);
                            continue;
                        }
                    };

                    if args.mode == 0 || args.mode == 1 {
                        let file_name = format!("{}.{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(), ext);
                        img::save_image(&mut content, &url, &file_name);
                    }
                    if (args.mode == 0 || args.mode == 2) && ["jpg", "jpeg", "png", "gif"].iter().any(|ext_i| ext.contains(ext_i)) {
                        if ext == "gif" { img::extract_middle_frame_from_gif(&mut content).await; }
                        let nsfw = match img::analyze_image(&mut content).await {
                            Ok(nsfw) => nsfw,
                            Err(error) => {
                                eprintln!("NSFW API error: {}", error);
                                continue;
                            }
                        };
                        if (nsfw.porn + nsfw.sexy).max(nsfw.hentai) > 0.8 {
                            let to_nick = nick + ", ";
                            let msg_chat;
                            if nsfw.porn.max(nsfw.sexy) > nsfw.hentai {
                                msg_chat = format!("{} {} ({}) {}", to_nick, texts.get_pron(), nsfw, texts.get_url());
                            } else {
                                msg_chat = format!("{} {} ({}) {}", to_nick, texts.get_anime(), nsfw, texts.get_url());
                            }
                            gg_chat(&write, Chat::Say(msg_chat));
                        }
                    }
                }
            } else {
                println!("_________ {:?} _________", parsed);
            }
        }
        // socket.close(None);
    }
}

fn gg_chat(write: &Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>, action: Chat) {
    let write = Arc::clone(&write);
    tokio::spawn(async move {
        let data: String;
        match action {
            Chat::Auth => data = format!("{{\"type\":\"auth\",\"data\":{{\"user_id\":\"{}\",\"token\":\"{}\"}}}}",
                                         env::var("GG_USER_ID").expect("GG_USER_ID must be set."),
                                         env::var("GG_TOKEN").expect("GG_TOKEN must be set."))
            ,
            Chat::Join => data = format!("{{\"type\":\"join\",\"data\":{{\"channel_id\":\"{}\",\"hidden\":false}}}}",
                                         env::var("GG_CHAT_ID").expect("GG_CHAT_ID must be set.")),
            Chat::Say(msg) => data = format!("{{\"type\":\"send_message\",\"data\":{{\"channel_id\":\"{}\",\"text\":\"{}\",\"hideIcon\":false,\"mobile\":false}}}}",
                                             env::var("GG_CHAT_ID").expect("GG_CHAT_ID must be set."), msg),
                                             //"172484".to_string(), msg),
            Chat::Ping => data = format!("{{\"type\":\"ping\",\"data\":{{}}}}"),
        }

        let mut write = write.lock().await;
        write.send(Message::Text(data)).await.expect("Error sending message");
    });
}
