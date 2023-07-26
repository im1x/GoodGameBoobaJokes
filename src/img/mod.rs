use std::io::{copy, Cursor, SeekFrom};
use bytes::Bytes;
use std::fs::File;
use image::{AnimationDecoder, DynamicImage};
use tokio::io::AsyncSeekExt;
use crate::api_client::{Nsfw, nsfw_test};

pub async fn get_image(url: &str) -> Result<Cursor<Bytes>, reqwest::Error> {
    let response = reqwest::get(url).await?;
    Ok(Cursor::new(response.bytes().await?))
}

pub fn save_image(content: &mut Cursor<Bytes>, url: &str, file_name: &str) {
    let mut out_file = File::create(format!("img/{}", file_name)).expect("failed to create file");
    copy(content, &mut out_file).expect("failed to copy content");
    println!("{} <-- {}", file_name, url);
}

pub async fn analyze_image(content: &mut Cursor<Bytes>) -> Result<Nsfw, Box<dyn std::error::Error>> {
    content.seek(SeekFrom::Start(0)).await.unwrap();
    nsfw_test(content).await
}

pub async fn extract_middle_frame_from_gif(content: &mut Cursor<Bytes>) {
    content.seek(SeekFrom::Start(0)).await.unwrap();
    let decoder = match image::codecs::gif::GifDecoder::new(&mut *content) {
        Ok(decoder) => decoder,
        Err(image::ImageError::Unsupported(_)) => return,
        Err(err) => {
           eprintln!("decoding of GIF failed with: {}", err);
            return;
        }
    };
    let mut frames = match decoder.into_frames().collect_frames() {
        Ok(frames) => frames,
        Err(image::ImageError::Unsupported(_)) => return,
        Err(err) => {
            eprintln!("collecting frames of GIF failed with: {}", err);
            return;
        }
    };

    // Select a single frame
    let frame = frames.drain(frames.len() / 2..).nth(0).unwrap();

    // Convert the frame to a`RgbaImage`
    let image_data = DynamicImage::from(frame.into_buffer());

    let mut jpeg_data = Cursor::new(Vec::new());
    image_data.write_to(&mut jpeg_data, image::ImageOutputFormat::Jpeg(85)).unwrap();

    let cursor = Cursor::new(Bytes::from(jpeg_data.into_inner()));
    *content = cursor;
}
