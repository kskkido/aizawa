use crate::sanity_client::{self, Mutation, PostImageAssetQuery};
use crate::store;
use futures::stream::{self, StreamExt, TryStreamExt};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

static SANITY_API_VERSION: &str = "2026-02-19";
const CONCURRENCY: usize = 3;

#[derive(Debug)]
pub struct Config {
    pub scope: SeederScope,
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

#[derive(Debug)]
pub enum SeederDocument {
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

impl Seeder {
    async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let store = store::Store::new(store::Config {
            base_dir: config.seed_dir.clone(),
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

    async fn seed(self: &Self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.scope {
            SeederScope::All => self.seed_profile().await?,
            SeederScope::Documents(documents) => {
                for document in documents.iter() {
                    match document {
                        SeederDocument::Profile => self.seed_profile().await?,
                        SeederDocument::Projects => self.seed_projects().await?,
                    }
                }
            }
        }
        Ok(())
    }

    async fn seed_profile(self: &Self) -> Result<(), Box<dyn std::error::Error>> {
        let profile_metadata = self.store.read_profile_metadata().await?;
        let image_asset = self
            .sanity
            .post_asset_image(
                PostImageAssetQuery {
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
            .post_mutate(&[Mutation::CreateOrReplace(serde_json::json!({
                "_id": "projects",
                "_type": "projects",
                "items": project_slugs.iter().map(|project_slug| serde_json::json!({
                  "_type": "reference",
                  "_ref": project_slug,
                  "_key": Uuid::now_v7().to_string(),
                })).collect::<Vec<_>>()
            }))])
            .await?;
        println!("Seeded projects: {:?}", project_slugs);
        Ok(())
    }

    async fn seed_project(&self, project_slug: &String) -> Result<(), Box<dyn std::error::Error>> {
        let project_metadata = self.store.read_project_metadata(project_slug).await?;
        let project_image_asset_ids = stream::iter(&project_metadata.images)
            .map(async |uri| -> Result<String, Box<dyn std::error::Error>> {
                let file = self.store.read_project_image(project_slug, uri).await?;
                let response = self
                    .sanity
                    .post_asset_image(
                        PostImageAssetQuery {
                            filename: Some(&uri),
                            title: Some(&project_metadata.title),
                            description: None,
                        },
                        file,
                    )
                    .await?;

                Ok(response.document._id)
            })
            .buffer_unordered(CONCURRENCY)
            .try_collect::<Vec<String>>()
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
                "rows": project_image_asset_ids
                    .iter()
                    .map(|asset_id| serde_json::json!({
                        "_type": "row",
                        "_key": Uuid::now_v7().to_string(),
                        "media": [{
                            "_type": "image",
                            "_key": Uuid::now_v7().to_string(),
                            "asset": {
                                "_type": "reference",
                                "_ref": asset_id,
                            },
                        }],
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
