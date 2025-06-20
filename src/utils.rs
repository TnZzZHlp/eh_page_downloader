use crate::{CLIENT, COOKIE, error};

async fn check() {
    let resp = CLIENT
        .get("https://exhentai.org/?f_search=cos%3A%22ringo+mitsuki%24%22+")
        .header("Cookie", &*COOKIE)
        .send()
        .await
        .expect("Failed to send request");

    if !resp.status().is_success() {
        error!("Failed to connect to the server: {}", resp.status());
    }
}
