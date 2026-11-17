use crate::sanity_client::{self, Mutation, PostAssetQuery};
use crate::store;
use clap::ValueEnum;
use futures::stream::{self, StreamExt, TryStreamExt};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

static SANITY_API_VERSION: &str = "2026-02-19";
const CONCURRENCY: usize = 3;

#[derive(Debug)]
pub struct Config {
    pub scope: SeederScope,
    pub data_dir: String,
    pub seed_dir: String,
    pub sanity_project_id: String,
    pub sanity_dataset: String,
    pub sanity_api_token: String,
}

#[derive(Debug)]
pub enum SeederScope {
    All,
    Documents(Vec<SeederDocument>),
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum SeederDocument {
    SiteSettings,
    Profile,
    Projects,
}

pub struct Seeder {
    store: store::Store,
    scope: SeederScope,
    sanity: Arc<sanity_client::Client>,
}

fn portable_text_block(children: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "_type": "block",
        "_key": Uuid::now_v7().to_string(),
        "style": "normal",
        "markDefs": [],
        "children": children,
    })
}

fn portable_text_span(text: &str) -> serde_json::Value {
    serde_json::json!({
        "_type": "span",
        "_key": Uuid::now_v7().to_string(),
        "text": text,
        "marks": [],
    })
}

fn portable_text_break() -> serde_json::Value {
    serde_json::json!({
        "_type": "break",
        "_key": Uuid::now_v7().to_string(),
    })
}

fn lines_to_portable_text(lines: Vec<String>) -> Vec<serde_json::Value> {
    let mut groups: Vec<Vec<String>> = vec![];
    let mut current: Vec<String> = vec![];

    for line in lines {
        if line.trim().is_empty() {
            groups.push(std::mem::take(&mut current));
        } else {
            current.push(line);
        }
    }

    if !current.is_empty() {
        groups.push(current);
    }

    groups
        .iter()
        .map(|group| {
            let children = group
                .iter()
                .enumerate()
                .flat_map(|(i, line)| {
                    if i == group.len() - 1 {
                        vec![portable_text_span(line)]
                    } else {
                        vec![portable_text_span(line), portable_text_break()]
                    }
                })
                .collect();
            portable_text_block(children)
        })
        .collect()
}

fn asset_reference(asset_id: &str) -> serde_json::Value {
    serde_json::json!({
        "_type": "reference",
        "_ref": asset_id,
    })
}

// an entry of a project row's `media` array
fn media_item(media: &store::Media, asset_ids: &HashMap<&String, String>) -> serde_json::Value {
    match media {
        store::Media::Image { file } => serde_json::json!({
            "_type": "image",
            "_key": Uuid::now_v7().to_string(),
            "asset": asset_reference(&asset_ids[file]),
        }),
        store::Media::Video { file, poster } => {
            let mut video = serde_json::json!({
                "_type": "video",
                "_key": Uuid::now_v7().to_string(),
                "asset": asset_reference(&asset_ids[file]),
            });
            if let Some(poster) = poster {
                video["poster"] = serde_json::json!({
                    "_type": "image",
                    "asset": asset_reference(&asset_ids[poster]),
                });
            }
            video
        }
    }
}

fn video_content_type(file_name: &str) -> Result<&'static str, Box<dyn std::error::Error>> {
    match Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some("mp4") => Ok("video/mp4"),
        _ => Err(format!("unsupported video file {file_name}").into()),
    }
}

impl Seeder {
    async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let store = store::Store::new(store::Config {
            data_dir: config.data_dir,
            seed_dir: config.seed_dir,
        })
        .await?;
        let sanity = sanity_client::Client::new(sanity_client::Config {
            project_id: config.sanity_project_id,
            dataset: config.sanity_dataset,
            api_token: config.sanity_api_token,
            api_version: SANITY_API_VERSION.to_string(),
        })?;

        Ok(Self {
            store,
            scope: config.scope,
            sanity: Arc::new(sanity),
        })
    }

    async fn seed(&self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.scope {
            SeederScope::All => {
                self.seed_site_settings().await?;
                self.seed_profile().await?;
                self.seed_projects().await?;
            }
            SeederScope::Documents(documents) => {
                for document in documents.iter() {
                    match document {
                        SeederDocument::Profile => self.seed_profile().await?,
                        SeederDocument::Projects => self.seed_projects().await?,
                        SeederDocument::SiteSettings => self.seed_site_settings().await?,
                    }
                }
            }
        }
        Ok(())
    }

    async fn seed_site_settings(&self) -> Result<(), Box<dyn std::error::Error>> {
        let site_settings = self.store.read_site_settings().await?;
        println!("{:?}", site_settings);
        self.sanity
            .post_mutate(&[Mutation::CreateOrReplace(serde_json::json!({
                    "_id": "site-settings",
                    "_type": "site-settings",
                    "title": site_settings.title,
                    "description": site_settings.description
            }))])
            .await
    }

    async fn seed_profile(&self) -> Result<(), Box<dyn std::error::Error>> {
        let profile_metadata = self.store.read_profile_metadata().await?;
        let image_asset = self
            .sanity
            .post_asset_image(
                PostAssetQuery {
                    filename: Some(&profile_metadata.image.url),
                    title: Some(&profile_metadata.image.alt),
                    description: Some(&profile_metadata.image.caption),
                },
                self.store
                    .read_profile_image(&profile_metadata.image.url)
                    .await?,
            )
            .await?;
        // createOrReplace writes the published document directly and overwrites it on reruns.
        self.sanity
            .post_mutate(&[Mutation::CreateOrReplace(serde_json::json!({
                "_id": "profile",
                "_type": "profile",
                "description": lines_to_portable_text(profile_metadata.description),
                "image": {
                    "_type": "image",
                    "asset": {
                        "_type": "reference",
                        "_ref": image_asset.document._id,
                    },
                },
            }))])
            .await?;
        println!("Created profile");
        Ok(())
    }

    async fn seed_projects(&self) -> Result<(), Box<dyn std::error::Error>> {
        let project_slugs = self.store.read_project_slugs().await?;
        let _ = self.delete_projects(&project_slugs).await;
        println!("Seeding projects: {:?}", project_slugs);
        let _ = stream::iter(&project_slugs)
            .map(|project_slug| self.seed_project(project_slug))
            .buffer_unordered(CONCURRENCY)
            .try_collect::<()>()
            .await?;
        self.sanity
            .post_mutate(&[
                Mutation::CreateOrReplace(serde_json::json!({
                    "_id": "projects",
                    "_type": "projects",
                    "items": project_slugs.iter().map(|project_slug| serde_json::json!({
                      "_type": "reference",
                      "_ref": project_slug,
                      "_key": Uuid::now_v7().to_string(),
                    })).collect::<Vec<_>>()
                })),
                Mutation::CreateOrReplace(serde_json::json!({
                    "_id": "home",
                    "_type": "home",
                    "slides": project_slugs.iter().take(5).map(|project_slug| serde_json::json!({
                      "_type": "reference",
                      "_ref": project_slug,
                      "_key": Uuid::now_v7().to_string(),
                    })).collect::<Vec<_>>()
                })),
            ])
            .await?;
        println!("Seeded projects: {:?}", project_slugs);
        Ok(())
    }

    async fn seed_project(&self, project_slug: &String) -> Result<(), Box<dyn std::error::Error>> {
        let project_metadata = self.store.read_project_metadata(project_slug).await?;
        // (file name, is video); a video's poster is uploaded as an image
        let uploads = project_metadata
            .rows
            .iter()
            .flatten()
            .flat_map(|media| match media {
                store::Media::Image { file } => vec![(file, false)],
                store::Media::Video { file, poster } => std::iter::once((file, true))
                    .chain(poster.iter().map(|poster| (poster, false)))
                    .collect(),
            });
        // uploads finish out of order, so map each file name to its asset id
        let asset_ids: HashMap<&String, String> = stream::iter(uploads)
            .map(
                async |(file_name, is_video)| -> Result<(&String, String), Box<dyn std::error::Error>> {
                    let body = self.store.read_project_image(project_slug, file_name).await?;
                    let query = PostAssetQuery {
                        filename: Some(file_name),
                        title: Some(&project_metadata.title),
                        description: None,
                    };
                    let response = if is_video {
                        self.sanity
                            .post_asset_file(query, video_content_type(file_name)?, body)
                            .await?
                    } else {
                        self.sanity.post_asset_image(query, body).await?
                    };

                    Ok((file_name, response.document._id))
                },
            )
            .buffer_unordered(CONCURRENCY)
            .try_collect()
            .await?;
        self.sanity
            .post_mutate(&[Mutation::CreateOrReplace(serde_json::json!({
                "_id": project_slug,
                "_type": "project",
                "title": project_metadata.title,
                "subtitle": project_metadata.subtitle,
                "slug": {
                    "_type": "slug",
                    "current": project_slug,
                },
                "content": lines_to_portable_text(project_metadata.description),
                "rows": project_metadata.rows
                    .iter()
                    .map(|row| serde_json::json!({
                        "_type": "row",
                        "_key": Uuid::now_v7().to_string(),
                        "media": row
                            .iter()
                            .map(|media| media_item(media, &asset_ids))
                            .collect::<Vec<_>>(),
                    }))
                    .collect::<Vec<_>>(),
            }))])
            .await?;
        println!("Created project {}", project_slug);
        Ok(())
    }

    async fn delete_projects(
        &self,
        project_slugs: &Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        stream::iter(project_slugs)
            .map(|project_slug| self.delete_project(project_slug))
            .buffer_unordered(CONCURRENCY)
            .try_collect::<Vec<()>>()
            .await?;
        Ok(())
    }

    async fn delete_project(&self, project_slug: &str) -> Result<(), Box<dyn std::error::Error>> {
        let project_ids: Vec<String> = self
            .sanity
            .post_query(
                "*[_type == 'project' && slug.current == $slug]._id",
                serde_json::json!({"slug": project_slug}),
            )
            .await?;

        for project_id in project_ids {
            let reference_ids: Vec<String> = self
                .sanity
                .post_query(
                    "*[references($id)]._id",
                    serde_json::json!({"id": project_id}),
                )
                .await?;
            let mut mutations: Vec<Mutation> = reference_ids
                .into_iter()
                .map(|reference_id| Mutation::Patch {
                    id: reference_id,
                    unset: vec!["slides".to_string(), "items".to_string()],
                })
                .collect();
            mutations.push(Mutation::Delete { id: project_id });
            self.sanity.post_mutate(&mutations).await?;
        }

        Ok(())
    }
}

pub async fn seed(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("Seeding with config: {:?}", config);
    let seeder = Seeder::new(config).await?;
    let output = seeder.seed().await?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_item_references_video_and_poster_assets() {
        let (file, poster) = ("a.mp4".to_string(), "a.jpg".to_string());
        let asset_ids = HashMap::from([
            (&file, "file-a".to_string()),
            (&poster, "image-a".to_string()),
        ]);
        let media = store::Media::Video {
            file: file.clone(),
            poster: Some(poster.clone()),
        };
        let mut item = media_item(&media, &asset_ids);
        item.as_object_mut().unwrap().remove("_key");
        assert_eq!(
            item,
            serde_json::json!({
                "_type": "video",
                "asset": { "_type": "reference", "_ref": "file-a" },
                "poster": {
                    "_type": "image",
                    "asset": { "_type": "reference", "_ref": "image-a" },
                },
            })
        );
    }

    #[test]
    fn video_item_without_poster_omits_poster() {
        let file = "a.mp4".to_string();
        let asset_ids = HashMap::from([(&file, "file-a".to_string())]);
        let media = store::Media::Video {
            file: file.clone(),
            poster: None,
        };
        assert!(media_item(&media, &asset_ids).get("poster").is_none());
    }

    #[test]
    fn non_mp4_video_is_rejected() {
        assert!(video_content_type("a.avi").is_err());
    }
}
