use anyhow::Result;
use rand::prelude::*;
use reqwest::Url;
use std::time::Duration;
use tokio::time::sleep;

use crate::utils::check;
use crate::{ARGS, CLIENT, info};

#[derive(Debug, Clone)]
pub struct Gallery {
    pub url: String,
    pub title: String,
    pub images: Vec<String>,
}

pub async fn parse_list(url: &str) -> Result<Vec<Gallery>> {
    let mut cur_page = url.to_string();
    let mut galleries = Vec::new();

    loop {
        info!("Fetching page: {}", cur_page);
        let (html, _) = check(&cur_page).await;

        let document = scraper::Html::parse_document(&html);

        // find gallery items
        let gallery_selector = scraper::Selector::parse(".gl2e").unwrap();

        for element in document.select(&gallery_selector) {
            let title = element
                .select(&scraper::Selector::parse(".glink").unwrap())
                .next()
                .unwrap()
                .text()
                .collect::<String>()
                .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "")
                .trim()
                .to_owned();

            let url = element
                .select(&scraper::Selector::parse(".gl2e > div > a").unwrap())
                .next()
                .unwrap()
                .value()
                .attr("href")
                .unwrap()
                .to_string();

            galleries.push(Gallery {
                url,
                title,
                images: Vec::new(),
            });
        }

        // find next page link
        let next_page_selector = scraper::Selector::parse("#dnext").unwrap();

        if let Some(next_page) = document.select(&next_page_selector).next() {
            if let Some(href) = next_page.value().attr("href") {
                cur_page = href.to_string();
            } else {
                break;
            }
        }

        sleep(Duration::from_millis(500)).await;
    }

    Ok(galleries)
}

pub async fn parse_gallery(gallery: &mut Gallery) -> Result<()> {
    info!("Parsing gallery: {}", gallery.title);
    // find image links
    let mut cur_url = gallery.url.clone();
    loop {
        let (html, _) = check(&cur_url).await;

        let document = scraper::Html::parse_document(&html);

        for element in document.select(&scraper::Selector::parse(".gt200 > a").unwrap()) {
            if let Some(href) = element.value().attr("href") {
                gallery.images.push(href.to_string());
            }
        }

        // find next page link
        if let Some(next_page) = document
            .select(
                &scraper::Selector::parse("table.ptt > tbody > tr > td:last-child > a").unwrap(),
            )
            .next()
        {
            if let Some(href) = next_page.value().attr("href") {
                if href != cur_url {
                    cur_url = href.to_string();
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }

        sleep(Duration::from_millis(rand::rng().random_range(500..=1000))).await;
    }

    Ok(())
}

pub async fn parse_real_image(image_page_url: &str) -> Result<String> {
    let (html, _) = check(image_page_url).await;

    if ARGS.original {
        let mut original_image_url = String::new();
        {
            // find the original image page URL
            let document = scraper::Html::parse_document(&html);
            let original_image_page_url = document
                .select(&scraper::Selector::parse("div#i6 div:last-child a").unwrap())
                .next()
                .and_then(|el| el.value().attr("href"));

            if let Some(url) = original_image_page_url {
                if let Ok(url) = Url::parse(url) {
                    original_image_url = url.to_string();
                }
            }
        }

        // Check if the URL is a redirection
        if !original_image_url.is_empty() {
            let (_, response_headers) = crate::utils::check(&original_image_url).await;

            let redirect_url = response_headers
                .get("Location")
                .and_then(|h| h.to_str().ok());
            if let Some(redirect_url) = redirect_url {
                return Ok(redirect_url.to_string());
            }
        }
    }

    // find the real image URL
    let document = scraper::Html::parse_document(&html);
    if let Some(img_element) = document
        .select(&scraper::Selector::parse("#img").unwrap())
        .next()
    {
        if let Some(src) = img_element.value().attr("src") {
            return Ok(src.to_string());
        }
    }

    Err(anyhow::anyhow!("Failed to find the real image URL"))
}
