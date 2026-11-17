use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::{fs, io::BufWriter};

static SITE_SETTINGS_PATH: &str = "site-settings.json";
static PROFILE_DIR: &str = "profile";
static PROJECTS_DIR: &str = "projects";

#[derive(Debug)]
pub struct Store {
    data_dir: PathBuf,
    seed_dir: PathBuf,
}

pub struct Config {
    pub data_dir: String,
    pub seed_dir: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SiteSettings {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProfileMetadata {
    pub name: String,
    pub description: Vec<String>,
    pub image: ImageMetadata,
    pub socials: SocialsMetadata,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImageMetadata {
    pub url: String,
    pub alt: String,
    pub caption: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SocialsMetadata {
    pub youtube: String,
    pub instagram: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectMetadata {
    pub title: String,
    pub subtitle: String,
    pub description: Vec<String>,
    // media grouped by the row they appear in on the page
    pub rows: Vec<Vec<Media>>,
}

// file names are relative to the project's seed directory
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Media {
    Image {
        file: String,
    },
    Video {
        file: String,
        poster: Option<String>,
    },
}

impl Store {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = PathBuf::from(config.data_dir);
        let seed_dir = PathBuf::from(config.seed_dir);
        fs::create_dir_all(&seed_dir).await?;
        Ok(Self { data_dir, seed_dir })
    }

    pub async fn read<T: DeserializeOwned>(
        path: impl AsRef<Path>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let bytes = fs::read_to_string(path).await?;
        let out = serde_json::from_str::<T>(&bytes)?;
        Ok(out)
    }

    pub async fn read_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let bytes = fs::read(path).await?;
        Ok(bytes)
    }

    pub async fn read_site_settings(&self) -> Result<SiteSettings, Box<dyn std::error::Error>> {
        Self::read::<SiteSettings>(self.site_settings_path()).await
    }

    pub async fn read_profile_image(
        &self,
        file_name: &String,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Self::read_bytes(self.prepend_profile_path(file_name)).await
    }

    pub async fn read_profile_metadata(
        &self,
    ) -> Result<ProfileMetadata, Box<dyn std::error::Error>> {
        self.read_project::<ProfileMetadata>(self.prepend_profile_path("metadata.json"))
            .await
    }

    pub async fn write_profile_metadata(
        &self,
        metadata: &ProfileMetadata,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(
            self.prepend_profile_path("metadata.json"),
            serde_json::to_string_pretty(metadata)?,
        )
        .await?;

        Ok(())
    }

    pub async fn write_profile_data(
        &self,
        file_name: &String,
        contents: impl AsRef<[u8]>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(self.prepend_profile_path(file_name), contents).await?;

        Ok(())
    }

    pub async fn read_project<T: DeserializeOwned>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        Self::read(self.prepend_project_path(path)).await
    }

    pub async fn read_project_metadata(
        &self,
        slug: impl AsRef<Path>,
    ) -> Result<ProjectMetadata, Box<dyn std::error::Error>> {
        let project_dir = self.prepend_project_path(slug);
        self.read_project::<ProjectMetadata>(project_dir.join("metadata.json"))
            .await
    }

    pub async fn read_project_image(
        &self,
        slug: impl AsRef<Path>,
        file_name: &String,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let project_dir = self.prepend_project_path(slug);
        Self::read_bytes(project_dir.join(file_name)).await
    }

    pub async fn write_project_metadata(
        &self,
        slug: impl AsRef<Path>,
        metadata: &ProjectMetadata,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(
            self.prepend_project_path(slug).join("metadata.json"),
            serde_json::to_string_pretty(metadata)?,
        )
        .await?;

        Ok(())
    }

    pub async fn open_project_file_writer(
        &self,
        slug: impl AsRef<Path>,
        file_name: impl AsRef<Path>,
    ) -> Result<BufWriter<fs::File>, Box<dyn std::error::Error>> {
        let file = fs::File::create(self.prepend_project_path(slug).join(file_name)).await?;

        Ok(BufWriter::new(file))
    }

    pub async fn read_project_slugs(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut project_slugs = vec![];
        let mut project_dir = fs::read_dir(self.projects_path()).await?;

        while let Some(entry) = project_dir.next_entry().await? {
            if let Some(project_slug) = entry.file_name().into_string().ok() {
                project_slugs.push(project_slug);
            }
        }

        Ok(project_slugs)
    }

    pub fn site_settings_path(&self) -> PathBuf {
        self.data_dir.join(SITE_SETTINGS_PATH)
    }

    pub fn profile_path(&self) -> PathBuf {
        self.seed_dir.join(PROFILE_DIR)
    }

    pub fn prepend_profile_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.profile_path().join(path)
    }

    pub fn projects_path(&self) -> PathBuf {
        self.seed_dir.join(PROJECTS_DIR)
    }

    pub fn prepend_project_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.projects_path().join(path)
    }
}
