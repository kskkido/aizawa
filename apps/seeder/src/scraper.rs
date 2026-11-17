use crate::store;
use clap::ValueEnum;
use scraper::{ElementRef, Html, Node, Selector};
use serde::Deserialize;
use std::{path::Path, sync::Arc};
use tokio::{io::AsyncWriteExt, sync, task};

#[derive(Debug)]
pub struct Config {
    pub scope: ScraperScope,
    pub data_dir: String,
    pub seed_dir: String,
    pub sitemap_url: String,
    pub allowed_list: Vec<String>,
    pub skip_paths: Vec<String>,
}

#[derive(Debug)]
pub enum ScraperScope {
    All,
    Documents(Vec<ScraperDocument>),
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ScraperDocument {
    Profile,
    Projects,
}

#[derive(Debug, Deserialize)]
struct SitemapUrl {
    #[serde(rename = "loc")]
    location: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "urlset")]
struct Sitemap {
    #[serde(rename = "url")]
    urls: Vec<SitemapUrl>,
}

#[derive(Debug)]
struct Scraper {
    config: Config,
    store: Arc<store::Store>,
    sitemap: Sitemap,
}

static PROFILE_PATH: &str = "about";
const MAX_IMAGES_PER_ROW: usize = 4;
// Adobe Creative Cloud Video, the player Adobe Portfolio uses for uploaded videos
static CCV_EMBED_DOMAIN: &str = "www-ccv.adobe.io";

#[derive(Clone, Debug)]
enum MediaSource {
    Image(url::Url),
    Video(url::Url),
}

fn media_rows(document: &Html, allowed_list: &[String]) -> Vec<Vec<MediaSource>> {
    let module_selector = Selector::parse("#project-modules > .project-module").unwrap();
    let image_selector = Selector::parse("img[data-src]").unwrap();
    let grid_selector = Selector::parse("[data-grid-max-images]").unwrap();
    let embed_selector = Selector::parse("iframe[src]").unwrap();

    let mut rows = Vec::new();
    for module in document.select(&module_selector) {
        if module.value().has_class(
            "project-module-video",
            scraper::CaseSensitivity::CaseSensitive,
        ) {
            for embed in module.select(&embed_selector) {
                let source = embed.attr("src").unwrap_or_default();
                match url::Url::parse(source) {
                    Ok(embed_url) if embed_url.domain() == Some(CCV_EMBED_DOMAIN) => {
                        rows.push(vec![MediaSource::Video(embed_url)]);
                    }
                    _ => println!("Skipping unsupported video embed {}", source),
                }
            }
            continue;
        }

        let images: Vec<MediaSource> = module
            .select(&image_selector)
            .filter_map(|el| el.attr("data-src"))
            .filter_map(|source| url::Url::parse(source).ok())
            .filter(|url| {
                url.domain()
                    .is_some_and(|domain| allowed_list.iter().any(|allowed| allowed == domain))
            })
            .map(MediaSource::Image)
            .collect();
        if images.is_empty() {
            continue;
        }
        let images_per_row = module
            .select(&grid_selector)
            .next()
            .and_then(|grid| grid.attr("data-grid-max-images"))
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(MAX_IMAGES_PER_ROW)
            .clamp(1, MAX_IMAGES_PER_ROW);
        rows.extend(images.chunks(images_per_row).map(|row| row.to_vec()));
    }
    rows
}

#[derive(Debug)]
struct CcvEmbed {
    mp4: url::Url,
    playlist: Option<url::Url>,
    poster: Option<url::Url>,
}

fn parse_ccv_embed(html: &str) -> Result<CcvEmbed, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html);
    let attr_url = |selector: &str, attr: &str| {
        document
            .select(&Selector::parse(selector).unwrap())
            .next()
            .and_then(|el| el.attr(attr))
            .and_then(|value| url::Url::parse(value).ok())
    };

    Ok(CcvEmbed {
        mp4: attr_url(r#"video source[type="video/mp4"]"#, "src")
            .ok_or("video player has no mp4 source")?,
        playlist: attr_url(r#"video source[type="application/x-mpegURL"]"#, "src"),
        poster: attr_url("video[data-poster]", "data-poster"),
    })
}

fn max_playlist_height(playlist: &str) -> Option<u32> {
    playlist
        .lines()
        .filter_map(|line| line.split("RESOLUTION=").nth(1))
        .filter_map(|resolution| {
            resolution
                .split(',')
                .next()?
                .split('x')
                .nth(1)?
                .parse()
                .ok()
        })
        .max()
}

fn mp4_rendition(mp4: &url::Url, height: u32) -> Option<url::Url> {
    let file_name = mp4.path_segments()?.next_back()?;
    let (id, _) = file_name.rsplit_once('_')?;
    let mut rendition = mp4.clone();
    rendition
        .path_segments_mut()
        .ok()?
        .pop()
        .push(&format!("{id}_{height}.mp4"));
    Some(rendition)
}

struct CcvVideo {
    file: url::Url,
    poster: Option<url::Url>,
}

async fn resolve_ccv_video(
    client: &reqwest::Client,
    embed_url: &url::Url,
) -> Result<CcvVideo, Box<dyn std::error::Error>> {
    let html = client
        .get(embed_url.clone())
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let embed = parse_ccv_embed(&html).map_err(|e| format!("{embed_url}: {e}"))?;
    let height = match &embed.playlist {
        Some(playlist_url) => {
            let playlist = client
                .get(playlist_url.clone())
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            max_playlist_height(&playlist)
        }
        None => None,
    };
    let file = height
        .and_then(|height| mp4_rendition(&embed.mp4, height))
        .unwrap_or(embed.mp4);
    Ok(CcvVideo {
        file,
        poster: embed.poster,
    })
}

fn file_name(url: &url::Url) -> String {
    Path::new(url.path())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap()
        .to_string()
}

fn text_lines(element: ElementRef) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for node in element.descendants() {
        match node.value() {
            Node::Text(text) => current.push_str(&text.replace('\u{a0}', " ")),
            Node::Element(el) if el.name() == "br" => {
                lines.push(std::mem::take(&mut current).trim().to_string());
            }
            _ => {}
        }
    }
    if !current.trim().is_empty() {
        lines.push(current.trim().to_string());
    }
    lines
}

impl Scraper {
    async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let store = store::Store::new(store::Config {
            data_dir: config.data_dir.clone(),
            seed_dir: config.seed_dir.clone(),
        })
        .await?;
        let client = reqwest::Client::new();
        let sitemap_text = client.get(&config.sitemap_url).send().await?.text().await?;
        let sitemap: Sitemap = quick_xml::de::from_str(&sitemap_text)?;

        Ok(Self {
            store: Arc::new(store),
            config,
            sitemap,
        })
    }

    async fn scrape(&self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.config.scope {
            ScraperScope::All => {
                tokio::try_join!(self.scrape_profile(), self.scrape_projects())?;
            }
            ScraperScope::Documents(documents) => {
                for document in documents.iter() {
                    match document {
                        ScraperDocument::Profile => self.scrape_profile().await?,
                        ScraperDocument::Projects => self.scrape_projects().await?,
                    }
                }
            }
        }
        Ok(())
    }

    async fn scrape_projects(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let mut tasks = task::JoinSet::new();

        for url in &self.sitemap.urls {
            let location = url.location.clone();
            let client = client.clone();
            tasks.spawn(async move {
                let text = client.get(&location).send().await?.text().await?;
                Ok::<(String, String), reqwest::Error>((location, text))
            });
        }

        let title_selector = Selector::parse("h1.title").unwrap();
        let subtitle_selector = Selector::parse("h1.title + div").unwrap();
        let description_selector = Selector::parse(".js-text-editable").unwrap();
        let semaphore = Arc::new(sync::Semaphore::new(10));
        let mut download_tasks = task::JoinSet::new();

        while let Some(result) = tasks.join_next().await {
            let (page_url, document_text) = result??;
            let page_url = url::Url::parse(&page_url)?;
            let project_slug = page_url.path().trim_matches('/');

            if self.config.skip_paths.contains(&project_slug.to_string()) {
                continue;
            }

            let document = Html::parse_document(&document_text);

            let mut rows = Vec::new();
            let mut download_urls = Vec::new();
            for source_row in media_rows(&document, &self.config.allowed_list) {
                let mut row = Vec::new();
                for source in source_row {
                    match source {
                        MediaSource::Image(image_url) => {
                            row.push(store::Media::Image {
                                file: file_name(&image_url),
                            });
                            download_urls.push(image_url);
                        }
                        MediaSource::Video(embed_url) => {
                            let video = resolve_ccv_video(&client, &embed_url).await?;
                            row.push(store::Media::Video {
                                file: file_name(&video.file),
                                poster: video.poster.as_ref().map(file_name),
                            });
                            download_urls.push(video.file);
                            download_urls.extend(video.poster);
                        }
                    }
                }
                rows.push(row);
            }

            let title = document
                .select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();
            let subtitle = document
                .select(&subtitle_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();
            let description = document
                .select(&description_selector)
                .next()
                .map(text_lines)
                .unwrap_or_default();
            let metadata = store::ProjectMetadata {
                title,
                subtitle,
                description,
                rows,
            };
            self.store
                .write_project_metadata(&project_slug, &metadata)
                .await?;

            let client = reqwest::Client::new();
            for media_url in download_urls {
                download_tasks.spawn({
                    let semaphore = semaphore.clone();
                    let project_slug = project_slug.to_string();
                    let client = client.clone();
                    let store = self.store.clone();

                    async move {
                        let _permit = semaphore.acquire().await?;
                        println!("Downloading {}", media_url);
                        let mut response = client
                            .get(media_url.clone())
                            .send()
                            .await?
                            .error_for_status()?;
                        let mut file_writer = store
                            .open_project_file_writer(
                                &project_slug,
                                &Path::new(media_url.path()).file_name().unwrap(),
                            )
                            .await
                            .map_err(|e| format!("failed to create bufwriter: {:?}", e))?;
                        while let Some(chunk) = response.chunk().await? {
                            file_writer.write_all(&chunk).await?;
                        }
                        file_writer.flush().await?;
                        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                    }
                });
            }
        }

        download_tasks.join_all().await;

        Ok(())
    }

    async fn scrape_profile(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = self
            .sitemap
            .urls
            .iter()
            .find(|url| {
                let url = url::Url::parse(&url.location).unwrap();
                let path = url.path().trim_matches('/');
                path == PROFILE_PATH
            })
            .ok_or("Profile page not found in sitemap")?;
        let text = client.get(&url.location).send().await?.text().await?;
        let document = Html::parse_document(&text);

        let name_selector = Selector::parse(".logo a").unwrap();
        let description_selector = Selector::parse(".module-text").unwrap();
        let image_selector = Selector::parse("section.page img[data-src]").unwrap();
        let caption_selector = Selector::parse(".module-caption").unwrap();
        let social_selector = Selector::parse(".social a[href]").unwrap();

        let name = document
            .select(&name_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let description = document
            .select(&description_selector)
            .next()
            .map(text_lines)
            .unwrap_or_default();

        let image_element = document
            .select(&image_selector)
            .find(|el| {
                el.attr("data-src")
                    .and_then(|source| url::Url::parse(source).ok())
                    .and_then(|url| url.domain().map(|d| d.to_owned()))
                    .is_some_and(|domain| {
                        self.config
                            .allowed_list
                            .iter()
                            .any(|allowed| domain.eq(allowed))
                    })
            })
            .ok_or("Profile image not found")?;
        let image_url = url::Url::parse(image_element.attr("data-src").unwrap_or_default())?;
        let image_file_name = Path::new(image_url.path())
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Profile image URL has no file name")?
            .to_string();
        let caption = document
            .select(&caption_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let social_links: Vec<&str> = document
            .select(&social_selector)
            .filter_map(|el| el.attr("href"))
            .collect();
        let find_social = |domain: &str| {
            social_links
                .iter()
                .find(|href| href.contains(domain))
                .map(|href| href.to_string())
                .unwrap_or_default()
        };

        let metadata = store::ProfileMetadata {
            image: store::ImageMetadata {
                url: image_file_name.clone(),
                alt: image_element.attr("alt").unwrap_or(&name).to_string(),
                caption,
            },
            socials: store::SocialsMetadata {
                youtube: find_social("youtube.com"),
                instagram: find_social("instagram.com"),
            },
            name,
            description,
        };

        self.store.write_profile_metadata(&metadata).await?;
        println!("Downloading {}", image_url);
        let image_bytes = client.get(image_url).send().await?.bytes().await?;
        self.store
            .write_profile_data(&image_file_name, image_bytes)
            .await?;

        Ok(())
    }
}

pub async fn scrape(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let scraper = Scraper::new(config).await?;
    let _ = scraper.scrape().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_of(modules: &str) -> Vec<Vec<String>> {
        let html =
            format!(r#"<section class="page"><div id="project-modules">{modules}</div></section>"#);
        let document = Html::parse_document(&html);
        media_rows(&document, &["cdn.example.com".to_string()])
            .iter()
            .map(|row| {
                row.iter()
                    .map(|source| match source {
                        MediaSource::Image(url) => file_name(url),
                        MediaSource::Video(url) => format!("video {url}"),
                    })
                    .collect()
            })
            .collect()
    }

    fn video_module(embed_url: &str) -> String {
        format!(
            r#"<div class="project-module module video project-module-video"><div class="embed-aspect-ratio"><iframe class="embed-content" src="{embed_url}"></iframe></div></div>"#
        )
    }

    fn image_module(name: &str) -> String {
        format!(
            r#"<div class="project-module module image"><img data-src="https://cdn.example.com/{name}"></div>"#
        )
    }

    fn grid_module(max_images: &str, names: &[&str]) -> String {
        let items: String = names
            .iter()
            .map(|name| {
                format!(
                    r#"<div class="grid__item-container"><img data-src="https://cdn.example.com/{name}"></div>"#
                )
            })
            .collect();
        format!(
            r#"<div class="project-module module media_collection"><div class="grid--main" data-grid-max-images="{max_images}">{items}</div></div>"#
        )
    }

    #[test]
    fn standalone_images_are_separate_rows() {
        let rows = rows_of(&(image_module("a.jpg") + &image_module("b.jpg")));
        assert_eq!(rows, vec![vec!["a.jpg"], vec!["b.jpg"]]);
    }

    #[test]
    fn tree_children_share_one_row() {
        let tree = format!(
            r#"<div class="project-module module tree"><div class="tree-child-wrapper">{}</div><div class="tree-child-wrapper">{}</div></div>"#,
            image_module("a.jpg"),
            image_module("b.jpg"),
        );
        assert_eq!(rows_of(&tree), vec![vec!["a.jpg", "b.jpg"]]);
    }

    #[test]
    fn grid_wraps_at_max_images() {
        let grid = grid_module("\n  2\n", &["a.jpg", "b.jpg", "c.jpg"]);
        assert_eq!(rows_of(&grid), vec![vec!["a.jpg", "b.jpg"], vec!["c.jpg"]]);
    }

    #[test]
    fn grid_rows_are_capped_at_schema_max() {
        let grid = grid_module("5", &["a.jpg", "b.jpg", "c.jpg", "d.jpg", "e.jpg"]);
        assert_eq!(
            rows_of(&grid),
            vec![vec!["a.jpg", "b.jpg", "c.jpg", "d.jpg"], vec!["e.jpg"]]
        );
    }

    #[test]
    fn images_from_other_domains_are_skipped() {
        let module = r#"<div class="project-module module image"><img data-src="https://other.example.com/a.jpg"></div>"#;
        assert!(rows_of(module).is_empty());
    }

    #[test]
    fn video_is_its_own_row_between_images() {
        let modules = image_module("a.jpg")
            + &video_module("https://www-ccv.adobe.io/v1/player/ccv/abc/embed")
            + &image_module("b.jpg");
        assert_eq!(
            rows_of(&modules),
            vec![
                vec!["a.jpg".to_string()],
                vec!["video https://www-ccv.adobe.io/v1/player/ccv/abc/embed".to_string()],
                vec!["b.jpg".to_string()],
            ]
        );
    }

    #[test]
    fn videos_from_other_players_are_skipped() {
        let module = video_module("https://www.youtube.com/embed/abc");
        assert!(rows_of(&module).is_empty());
    }

    #[test]
    fn ccv_embed_provides_mp4_playlist_and_poster() {
        let html = r#"<video id="video-js" data-poster="https://cdn.example.com/abc/image/abc_poster.jpg?t=1">
            <source src="https://cdn.example.com/abc/rend/master.m3u8?t=1" type="application/x-mpegURL">
            <source src="https://cdn.example.com/abc/rend/abc_576.mp4?t=1" type="video/mp4">
        </video>"#;
        let embed = parse_ccv_embed(html).unwrap();
        assert_eq!(
            (
                embed.mp4.as_str(),
                embed.playlist.as_ref().map(url::Url::as_str),
                embed.poster.as_ref().map(url::Url::as_str),
            ),
            (
                "https://cdn.example.com/abc/rend/abc_576.mp4?t=1",
                Some("https://cdn.example.com/abc/rend/master.m3u8?t=1"),
                Some("https://cdn.example.com/abc/image/abc_poster.jpg?t=1"),
            )
        );
    }

    #[test]
    fn ccv_embed_without_mp4_is_an_error() {
        assert!(parse_ccv_embed(r#"<video id="video-js"></video>"#).is_err());
    }

    #[test]
    fn playlist_max_height_is_the_largest_resolution() {
        let playlist = "#EXTM3U\n\
            #EXT-X-STREAM-INF:PROGRAM-ID=1,BANDWIDTH=1664000,RESOLUTION=1024x576\n\
            hls_576/index.m3u8\n\
            #EXT-X-STREAM-INF:PROGRAM-ID=1,BANDWIDTH=4424000,RESOLUTION=1920x1080\n\
            hls_1080/index.m3u8\n\
            #EXT-X-STREAM-INF:PROGRAM-ID=1,BANDWIDTH=2261000,RESOLUTION=1280x720\n\
            hls_720/index.m3u8\n";
        assert_eq!(max_playlist_height(playlist), Some(1080));
    }

    #[test]
    fn mp4_rendition_swaps_height_and_keeps_token() {
        let mp4 = url::Url::parse("https://cdn.example.com/abc/rend/abc_576.mp4?t=1").unwrap();
        assert_eq!(
            mp4_rendition(&mp4, 1080).unwrap().as_str(),
            "https://cdn.example.com/abc/rend/abc_1080.mp4?t=1"
        );
    }
}
