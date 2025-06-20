use std::time::Duration;

use regex::Regex;
use tokio::time::sleep;

use crate::{CLIENT, COOKIE, error};

async fn check() {
    let html = CLIENT
        .get("https://exhentai.org")
        .header("Cookie", &*COOKIE)
        .send()
        .await
        .expect("Failed to send request")
        .text()
        .await
        .expect("Failed to read response text");

    if !html
        .contains("This IP address has been temporarily banned due to an excessive request rate")
    {
        return;
    }

    let re = Regex::new(r"(\d+)\s*minutes?\s*and\s*(\d+)\s*seconds?").unwrap();
    if let Some(caps) = re.captures(&html) {
        let minutes: u64 = caps[1].parse().unwrap();
        let seconds: u64 = caps[2].parse().unwrap();
        sleep(Duration::from_secs(minutes * 60 + seconds)).await;
    }
}
