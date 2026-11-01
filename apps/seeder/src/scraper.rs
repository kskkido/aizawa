use crate::store;
use scraper::{ElementRef, Html, Node, Selector};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use tokio::{
    fs,
    io::{AsyncWriteExt, BufWriter},
    sync, task,
};

#[derive(Debug)]
pub struct Config {
    pub scope: ScraperScope,
    pub output_dir: String,
    pub sitemap_url: String,
    pub allowed_list: Vec<String>,
    pub skip_paths: Vec<String>,
}

#[derive(Debug)]
pub enum ScraperScope {
    All,
    Documents(Vec<ScraperDocument>),
}

#[derive(Debug)]
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

#[derive(Serialize)]
struct ProjectMetadata {
    title: String,
    subtitle: String,
    description: String,
    images: Vec<String>,
}

#[derive(Serialize)]
struct ProfileMetadata {
    name: String,
    description: Vec<String>,
    image: ImageMetadata,
    socials: SocialsMetadata,
}

#[derive(Deserialize, Serialize)]
struct ImageMetadata {
    url: String,
    alt: String,
    caption: String,
}

#[derive(Deserialize, Serialize)]
struct SocialsMetadata {
    youtube: String,
    instagram: String,
}

#[derive(Debug)]
struct Context {
    config: Config,
    store: Arc<store::Store>,
    sitemap: Sitemap,
}

static PROFILE_PATH: &str = "about";

impl Context {
    async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let store = store::Store::new(store::Config {
            base_dir: config.output_dir.clone(),
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
}

/// Collects an element's text as lines, starting a new line at each <br>.
/// The site uses &nbsp; between words; these become plain spaces.
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

async fn scrape_projects(context: &Context) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut tasks = task::JoinSet::new();

    for url in &context.sitemap.urls {
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
    let image_selector = Selector::parse("section.page img[data-src]").unwrap();
    let semaphore = Arc::new(sync::Semaphore::new(10));
    let mut download_tasks = task::JoinSet::new();

    while let Some(result) = tasks.join_next().await {
        let (page_url, document_text) = result??;
        let page_url = url::Url::parse(&page_url)?;
        let project_slug = page_url.path().trim_matches('/');

        if context
            .config
            .skip_paths
            .contains(&project_slug.to_string())
        {
            continue;
        }

        let document = Html::parse_document(&document_text);

        let mut image_urls = Vec::new();
        for element in document.select(&image_selector) {
            if let Some(source) = element.attr("data-src") {
                let image_url = match url::Url::parse(source) {
                    Ok(image_url) => image_url,
                    _ => continue,
                };
                let mut allowed = false;
                for entry in &context.config.allowed_list {
                    if image_url.domain() == Some(entry) {
                        allowed = true;
                    }
                }
                if !allowed {
                    continue;
                }
                image_urls.push(image_url);
            }
        }

        let images: Vec<String> = image_urls
            .iter()
            .map(|url| {
                Path::new(url.path())
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap()
                    .to_string()
            })
            .collect();

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
            images,
        };
        context
            .store
            .write_project_metadata(&project_slug, &metadata)
            .await?;

        let client = reqwest::Client::new();
        for image_url in image_urls.into_iter() {
            download_tasks.spawn({
                let semaphore = semaphore.clone();
                let project_slug = project_slug.to_string();
                let client = client.clone();
                let store = context.store.clone();

                async move {
                    let _permit = semaphore.acquire().await?;
                    println!("Downloading {}", image_url);
                    let mut response = client
                        .get(image_url.clone())
                        .send()
                        .await?
                        .error_for_status()?;
                    let mut file_writer = store
                        .open_project_file_writer(
                            &project_slug,
                            &Path::new(image_url.path()).file_name().unwrap(),
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

async fn scrape_profile(context: &Context) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = context
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
                    context
                        .config
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

    context.store.write_profile_metadata(&metadata).await?;
    println!("Downloading {}", image_url);
    let image_bytes = client.get(image_url).send().await?.bytes().await?;
    context
        .store
        .write_profile_data(&image_file_name, image_bytes)
        .await?;

    Ok(())
}

pub async fn scrape(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new(config).await?;
    match &context.config.scope {
        ScraperScope::All => {
            scrape_profile(&context).await?;
            scrape_projects(&context).await?;
        }
        ScraperScope::Documents(documents) => {
            for document in documents.iter() {
                match document {
                    ScraperDocument::Profile => scrape_profile(&context).await?,
                    ScraperDocument::Projects => scrape_projects(&context).await?,
                }
            }
        }
    }
    Ok(())
}
