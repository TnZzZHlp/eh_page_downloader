use anyhow::{Ok, Result};
use clap::Parser;
use rand::prelude::*;
use reqwest::Client;
use reqwest::redirect::Policy;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::time::sleep;

mod download;
mod macros;
mod parse;
mod utils;

#[derive(Parser, Debug)]
struct Cli {
    url: String,

    #[clap(long, default_value = "1")]
    concurrency: usize,

    #[clap(short, long)]
    cookie: String,

    #[clap(short, long, default_value = "output")]
    output: String,

    #[clap(long, default_value = "false")]
    original: bool,
}

static CLIENT: LazyLock<ClientWithMiddleware> = LazyLock::new(|| {
    ClientBuilder::new(Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36 Edg/137.0.0.0")
        .redirect(Policy::none())
        .build()
        .expect("Failed to create HTTP client"))
        // Retry failed requests.
        .with(RetryTransientMiddleware::new_with_policy(ExponentialBackoff::builder().build_with_max_retries(3)))
        .build()
});
static COOKIE: LazyLock<String> = LazyLock::new(|| Cli::parse().cookie);
static SEM: LazyLock<tokio::sync::Semaphore> =
    LazyLock::new(|| tokio::sync::Semaphore::new(Cli::parse().concurrency));
static OUTPUT: LazyLock<String> = LazyLock::new(|| Cli::parse().output);
static PB: LazyLock<indicatif::MultiProgress> = LazyLock::new(indicatif::MultiProgress::new);
static ORIGINAL: LazyLock<bool> = LazyLock::new(|| Cli::parse().original);

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    run(cli.url).await?;

    Ok(())
}

async fn run(url: String) -> Result<()> {
    let mut galleries = parse::parse_list(&url).await?;

    let pb = PB.add(indicatif::ProgressBar::new(galleries.len() as u64));
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] [{pos}/{len}] {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    for gallery in &mut galleries {
        parse::parse_gallery(gallery).await?;
        sleep(Duration::from_millis(rand::rng().random_range(500..=1000))).await;
        pb.inc(1);
    }

    pb.finish_with_message("All galleries parsed");

    info!("Starting downloads...");
    for gallery in galleries {
        download::download_gallery(gallery).await?;
    }

    Ok(())
}
