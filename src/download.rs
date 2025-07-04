use crate::{
    ARGS, CLIENT, PB, error, info, new_progress_bar,
    parse::{self, Gallery},
};
use anyhow::Result;
use std::{path::PathBuf, sync::Arc};
use tokio::{io::AsyncWriteExt, task::JoinSet};

pub async fn download_gallery(gallery: Gallery) -> Result<()> {
    let pb = Arc::new(PB.add(new_progress_bar(gallery.images.len() as u64)));

    let mut tasks = JoinSet::new();
    let title = Arc::new(gallery.title);
    info!("Downloading gallery: {}", title);
    for (index, image_url) in gallery.images.into_iter().enumerate() {
        let title = Arc::clone(&title);
        let pb = Arc::clone(&pb);
        tasks.spawn(async move {
            if let Err(e) = download_image(&image_url, &title, index).await {
                error!("Failed to download image {}: {}", index + 1, e);
            }
            pb.inc(1);
        });
    }
    tasks.join_all().await;
    pb.finish_and_clear();
    Ok(())
}

pub async fn download_image(image_url: &str, title: &str, index: usize) -> Result<()> {
    let _permit = crate::SEM.acquire().await;
    let img_url = parse::parse_real_image(image_url).await?;
    let ext = img_url
        .rsplit('.')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine file extension"))?;

    let output_path = PathBuf::from(format!("{}/{}/{}.{}", ARGS.output, title, index, ext));

    if output_path.exists() {
        info!("File already exists: {}", output_path.display());
        return Ok(());
    }

    if !output_path.parent().unwrap().exists() {
        std::fs::create_dir_all(output_path.parent().unwrap())?;
    }

    let response = CLIENT.get(&img_url).send().await?;

    if !response.status().is_success() {
        error!(
            "Failed to download image: {} - Status: {}",
            img_url,
            response.status()
        );
        return Err(anyhow::anyhow!(
            "Failed to download image: {}",
            response.status()
        ));
    }

    let mut file = tokio::fs::File::create(&output_path).await?;
    let content = response.bytes().await?;
    file.write_all(&content).await?;

    Ok(())
}
