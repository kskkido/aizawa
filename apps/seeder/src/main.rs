mod sanity_client;
mod scraper;
mod seeder;
mod store;
mod utils;

use clap::{Parser, Subcommand, ValueEnum};
use std::env;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version)]
struct CliArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Scrape {
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        all: Option<bool>,
        #[arg(long, value_enum, value_delimiter = ',')]
        document: Option<Vec<DocumentOption>>,
    },
    Seed {
        #[arg(long)]
        all: Option<bool>,
        #[arg(long, value_enum, value_delimiter = ',')]
        document: Option<Vec<DocumentOption>>,
        #[arg(long)]
        seed_dir: Option<String>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum DocumentOption {
    Profile,
    Projects,
}

impl From<DocumentOption> for scraper::ScraperDocument {
    fn from(arg: DocumentOption) -> Self {
        match arg {
            DocumentOption::Projects => scraper::ScraperDocument::Projects,
            DocumentOption::Profile => scraper::ScraperDocument::Profile,
        }
    }
}

impl From<DocumentOption> for seeder::SeederDocument {
    fn from(arg: DocumentOption) -> Self {
        match arg {
            DocumentOption::Projects => seeder::SeederDocument::Projects,
            DocumentOption::Profile => seeder::SeederDocument::Profile,
        }
    }
}

static SITEMAP_URL: &str = "https://deanaizawa.com/sitemap.xml";
static ALLOWED_LIST: [&str; 1] = ["cdn.myportfolio.com"];
static SKIP_PATHS: [&str; 3] = ["", "works", "about"];
static DEFAULT_SEED_DIR: &str = "tmp/seeds";

fn default_seed_dir() -> Result<String, Box<dyn std::error::Error>> {
    let root_path = utils::get_root_path()?;
    let mut path = PathBuf::new();
    path.push(root_path);
    path.push(DEFAULT_SEED_DIR);
    path.to_str()
        .ok_or_else(|| "Failed to convert path to str".into())
        .map(|str| str.to_string())
}

async fn run(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        Command::Scrape {
            all,
            document: document_list,
            output,
        } => {
            let scope = if all.unwrap_or(false) {
                scraper::ScraperScope::All
            } else if let Some(documents) = document_list {
                scraper::ScraperScope::Documents(documents.into_iter().map(Into::into).collect())
            } else {
                return Err("Either --all or --document must be specified".into());
            };
            scraper::scrape(scraper::Config {
                scope: scope,
                output_dir: output.unwrap_or_else(|| default_seed_dir().unwrap()),
                sitemap_url: SITEMAP_URL.to_string(),
                allowed_list: ALLOWED_LIST.iter().map(|s| s.to_string()).collect(),
                skip_paths: SKIP_PATHS.iter().map(|s| s.to_string()).collect(),
            })
            .await
        }
        Command::Seed {
            all,
            seed_dir,
            document: document_list,
        } => {
            let scope = if all.unwrap_or(false) {
                seeder::SeederScope::All
            } else if let Some(documents) = document_list {
                seeder::SeederScope::Documents(documents.into_iter().map(Into::into).collect())
            } else {
                return Err("Either --all or --document must be specified".into());
            };
            seeder::seed(seeder::Config {
                scope,
                seed_dir: seed_dir.unwrap_or_else(|| default_seed_dir().unwrap()),
                sanity_project_id: env::var("SANITY_STUDIO_PROJECT_ID")?,
                sanity_dataset: env::var("SANITY_STUDIO_DATASET")?,
                sanity_api_token: env::var("SANITY_API_TOKEN")?,
            })
            .await
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli_args = CliArgs::parse();
    run(cli_args).await
}
