use std::fs::File;
use std::io::{BufRead, BufReader, Error};
use std::path::Path;

use regex::Regex;

pub fn get_ext_from_url(url: &String) -> String {
    let ext = if let Some(ext_os_str) = Path::new(&url).extension() {
        let mut ext_temp = ext_os_str.to_str().unwrap().to_string();
        if let Some(pos) = ext_temp.find("?") {
            ext_temp = ext_temp[..pos].to_string();
        }
        ext_temp
    } else {
        String::new()
    };
    ext
}


pub fn get_urls(text: &str) -> String {
    let re = Regex::new(r#"(?:(http|https)://)?[\w-]+(\.[\w-]+)+([\w.,@?^=%&:/~+#-]*[\w@?^=%&/~+#-])?"#).unwrap();
    match re.captures(text) {
        Some(caps) => caps[0].to_string(),
        None => "".to_string(),
    }
}

pub fn read_file_lines(path: &str) -> Result<Vec<String>, Error> {
    BufReader::new(File::open(path)?).lines().collect()
}

#[test]
fn test_get_ext_from_url() {
    assert_eq!(get_ext_from_url(&String::from("https://ya.ru/img.jpg")), "jpg");
    assert_eq!(get_ext_from_url(&String::from("https://ya.ru/img.jpg?abc=123")), "jpg");
    assert_eq!(get_ext_from_url(&String::from("https://ya.ru/img.jpeg?abc=123&def=345")), "jpeg");
}

#[test]
fn test_get_urls() {
    assert_eq!(get_urls("aaaaaaaaaaaaaaa https://ya.ru/img.jpg"), "https://ya.ru/img.jpg");
    assert_eq!(get_urls("aaaaaaaaaaaaaaa https://ya.ru/img.jpg bbb "), "https://ya.ru/img.jpg");
    assert_eq!(get_urls("aaaaaaaaaaaaaaa https://ya.ru/img.jpg?abc=123"), "https://ya.ru/img.jpg?abc=123");
}
