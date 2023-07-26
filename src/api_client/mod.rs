use std::{env, fmt, io};
use std::io::Cursor;

use bytes::Bytes;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Nsfw {
    pub porn: f64,
    pub sexy: f64,
    pub hentai: f64,
    pub neutral: f64,
    pub drawing: f64,
}

impl fmt::Display for Nsfw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const THRESHOLD: f64 = 0.15;

        let mut output = String::new();

        if self.porn > THRESHOLD {
            output += &format!("porn: {:.0}%, ", self.porn * 100.0);
        }
        if self.sexy > THRESHOLD {
            output += &format!("sexy: {:.0}%, ", self.sexy * 100.0);
        }
        if self.hentai > THRESHOLD {
            output += &format!("hentai: {:.0}%, ", self.hentai * 100.0);
        }
        if self.neutral > THRESHOLD {
            output += &format!("neutral: {:.0}%, ", self.neutral * 100.0);
        }
        if self.drawing > THRESHOLD {
            output += &format!("drawing: {:.0}%, ", self.drawing * 100.0);
        }

        // Remove the trailing comma and space
        if output.len() > 2 {
            output.pop();
            output.pop();
        }

        write!(f, "{}", output)
    }
}

pub async fn nsfw_test(file: &mut Cursor<Bytes>) -> Result<Nsfw, Box<dyn std::error::Error>> {
    let api_url: String = env::var("NSFW_API_URL").expect("NSFW_API_URL must be set.");

    let client = reqwest::Client::new();

    let mut file_contents = Vec::new();
    io::copy(file, &mut file_contents)?;

    let mut form = reqwest::multipart::Form::new();
    let image_part = reqwest::multipart::Part::bytes(file_contents)
        .file_name("123.jpg".to_owned());
    form = form.part("image", image_part);

    let resp = client
        .post(api_url)
        .multipart(form)
        .send()
        .await?;

    if resp.status() != StatusCode::OK {
        return Err(format!("Request failed with status code {}", resp.status()).into());
    }

    let json_body: Nsfw = resp.json::<Nsfw>().await?;
    println!("{}", json_body);
    Ok(json_body)
}
