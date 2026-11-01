mod sanity_client;
mod scraper;
mod seeder;
mod store;
mod utils;

use clap::{Parser, Subcommand};
use std::env;
use std::path::{Path, PathBuf};

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
        document: Option<Vec<scraper::ScraperDocument>>,
    },
    Seed {
        #[arg(long)]
        all: Option<bool>,
        #[arg(long, value_enum, value_delimiter = ',')]
        document: Option<Vec<seeder::SeederDocument>>,
        #[arg(long)]
        seed_dir: Option<String>,
    },
}

static SITEMAP_URL: &str = "https://deanaizawa.com/sitemap.xml";
static ALLOWED_LIST: [&str; 1] = ["cdn.myportfolio.com"];
static SKIP_PATHS: [&str; 3] = ["", "works", "about"];
static DEFAULT_SEED_DIR: &str = "tmp/seeds";
static ROOT_DIR: &str = env!("CARGO_MANIFEST_DIR");

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
    let data_dir = Path::new(ROOT_DIR)
        .join("data")
        .to_str()
        .unwrap()
        .to_string();

    match args.command {
        Command::Scrape {
            all,
            document: document_list,
            output,
        } => {
            let scope = if all.unwrap_or(false) {
                scraper::ScraperScope::All
            } else if let Some(documents) = document_list {
                scraper::ScraperScope::Documents(documents)
            } else {
                return Err("Either --all or --document must be specified".into());
            };
            scraper::scrape(scraper::Config {
                scope,
                data_dir,
                seed_dir: output.unwrap_or_else(|| default_seed_dir().unwrap()),
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
                seeder::SeederScope::Documents(documents)
            } else {
                return Err("Either --all or --document must be specified".into());
            };
            seeder::seed(seeder::Config {
                scope,
                data_dir,
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
