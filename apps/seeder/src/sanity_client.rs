use std::time::Duration;

use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug)]
pub struct Client {
    http: reqwest::Client,
    base_url: reqwest::Url,
    config: Config,
}

#[derive(Debug)]
pub struct Config {
    pub project_id: String,
    pub dataset: String,
    pub api_token: String,
    pub api_version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Mutation {
    Patch { id: String, unset: Vec<String> },
    Delete { id: String },
    CreateOrReplace(serde_json::Value),
}

#[derive(Serialize)]
struct PostMutateBody<'a> {
    mutations: &'a [Mutation],
}

#[derive(Serialize)]
pub struct PostImageAssetQuery<'a> {
    pub filename: Option<&'a str>,
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
}

#[derive(Deserialize)]
pub struct PostImageAssetResponse {
    pub document: ImageAssetDocument,
}

#[derive(Deserialize)]
pub struct ImageAssetDocument {
    pub _id: String,
    pub _type: String,
}

#[derive(Serialize)]
pub struct PostQueryBody<'a, T> {
    query: &'a str,
    params: T,
}

#[derive(Deserialize)]
pub struct PostQueryResponse<T> {
    result: T,
}

static MAX_RETRIES: u32 = 5;

impl Client {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let raw_url = format!(
            "https://{}.api.sanity.io/v{}/",
            config.project_id, config.api_version
        );
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()?;
        let base_url = reqwest::Url::parse(&raw_url)?;

        Ok(Self {
            http,
            base_url,
            config,
        })
    }

    pub async fn post_mutate<'a>(
        &self,
        mutations: &[Mutation],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request = self
            .post(&format!("data/mutate/{}", self.config.dataset))?
            .json(&PostMutateBody { mutations });

        self.send::<serde_json::Value>(request).await?;

        Ok(())
    }

    pub async fn post_asset_image<'a>(
        &self,
        query: PostImageAssetQuery<'a>,
        body: Vec<u8>,
    ) -> Result<PostImageAssetResponse, Box<dyn std::error::Error>> {
        let request = self
            .post(&format!("assets/images/{}", self.config.dataset))?
            .query(&query)
            .body(body);
        let response = self.send::<PostImageAssetResponse>(request).await?;

        Ok(response)
    }

    pub async fn post_query<T: DeserializeOwned>(
        &self,
        query: &str,
        params: impl Serialize,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let request = self
            .post(&format!("data/query/{}", self.config.dataset))?
            .json(&PostQueryBody { query, params })
            .header("accept", "application/json");
        let response = self.send::<PostQueryResponse<T>>(request).await?;

        Ok(response.result)
    }

    fn post(&self, path: &str) -> Result<RequestBuilder, Box<dyn std::error::Error>> {
        self.url(path).map(|url| self.http.post(url))
    }

    fn url(&self, path: &str) -> Result<reqwest::Url, Box<dyn std::error::Error>> {
        let path_segments = path.split('/').filter(|s| !s.is_empty());
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| "base URL cannot have path segments")?
            .pop_if_empty()
            .extend(path_segments);
        Ok(url)
    }

    async fn send<T: DeserializeOwned>(
        &self,
        req: RequestBuilder,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let mut attempt = 0;
        loop {
            let request = req
                .try_clone()
                .ok_or("request can't be retried")?
                .bearer_auth(&self.config.api_token)
                .build()?;
            println!("Sending {} request to {}", request.method(), request.url());
            let response = match self.http.execute(request).await {
                Err(error) if error.is_timeout() && attempt < MAX_RETRIES => {
                    let wait_seconds = 2u64.pow(attempt);
                    tokio::time::sleep(Duration::from_secs(wait_seconds)).await;
                    attempt += 1;
                    continue;
                }
                Ok(response)
                    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                        && attempt < MAX_RETRIES =>
                {
                    let wait_seconds = response
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(2u64.pow(attempt));
                    println!("Attempt {}, sleeping for {} seconds", attempt, wait_seconds);
                    tokio::time::sleep(Duration::from_secs(wait_seconds)).await;
                    attempt += 1;
                    continue;
                }
                result => result?,
            };
            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(format!("{status}: {body}").into());
            }
            return Ok(response.json::<T>().await?);
        }
    }
}
