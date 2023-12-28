use std::{error::Error};
use std::process::exit;
use itertools::Either::Left;
use itertools::Either::Right;
use itertools::Itertools;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;
use clap::Parser;

#[derive(Parser, Default, Debug)]
#[clap(author="Duncan Lew", version, about)]
/// A Readwise to Omnivore importer
struct Arguments {
    #[clap(short, long)]
    /// API key for Omnivore
    key: String,

    #[clap(short, long)]
    /// File path for the CSV file
    file_path: String,
}

#[derive(Debug, serde::Deserialize)]
struct Article {
    #[serde(rename(deserialize = "Title"))]
    title: String,
    #[serde(rename(deserialize = "URL"))]
    url: String,
    #[serde(rename(deserialize = "Document tags"))]
    document_tags: String,
    #[serde(rename(deserialize = "Saved date"))]
    saved_date: String,
    #[serde(rename(deserialize = "Reading progress"))]
    reading_progress: String,
    #[serde(rename(deserialize = "Location"))]
    location: String,
    #[serde(rename(deserialize = "Seen"))]
    seen: String,
}

fn get_imported_articles(file_path: String) -> Result<(Vec<Article>), Box<dyn Error>> {
    let mut csv_reader = csv::Reader::from_path(file_path)?;
    let (errors, articles): (Vec<csv::Error>, Vec<Article>) = csv_reader
        .deserialize()
        .partition_map(|row| match row {
            Err(e) => Left(e),
            Ok(article) => Right(article)
        });

    if errors.is_empty() {
        Ok(articles)
    } else {
        Err("Errors occurred while reading the CSV".into())
    }
}

async fn save_url(key: String, article_url: String, saved_date: String, is_archived: bool) -> Result<(), Box<dyn Error>> {
    let mut input_map = serde_json::Map::new();

    input_map.insert("clientRequestId".to_string(), Value::String(format!("{}", Uuid::new_v4())));
    input_map.insert("source".to_string(), Value::String("api".to_string()));
    input_map.insert("url".to_string(), Value::String(format!("{}", article_url)));
    // TODO place this back
    // input_map.insert("savedAt".to_string(), Value::String(format!("{}", saved_date)));
    input_map.insert("labels".to_string(), json!([{"name": "imported"}]));
    if is_archived {
        input_map.insert("state".to_string(), Value::String("ARCHIVED".to_string()));
    }

    let payload = json!({
        "query": "mutation SaveUrl($input: SaveUrlInput!) { saveUrl(input: $input) { ... on SaveSuccess { url clientRequestId } ... on SaveError { errorCodes message } } }",
        "variables": {
            "input": input_map
        }
    });

    // println!("Payload");
    // println!("{}", payload.to_string());

    let client = reqwest::Client::new();
    let result = client.post("https://api-prod.omnivore.app/api/graphql")
        .json(&payload)
        .header("content-type", "application/json")
        .header("authorization", key)
        .send()
        .await;

    match result {
        Ok(response) => {
            if response.status().is_success() {
                // TODO remove these two lines at the end
                let result_body = response.text().await?;
                println!("Resulting body {:#?}", result_body);
                Ok(())
            } else {
                let status = response.status();
                let text = response.text().await?;
                let error_message = format!("Server returned the code \"{}\" and the message {}", status, text);
                Err(error_message.into())
            }
        }
        Err(error) => {
            let error_message = format!("Error while processing request: {}", error);
            Err(error_message.into())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();

    let imported_articles = get_imported_articles(arguments.file_path)
        .unwrap_or_else(|err| {
            eprintln!("{}", err);
            exit(1);
        });

    // println!("Doing this from the main");
    // imported_articles.iter().for_each(|article: &Article| println!("{:#?}", article));

    let article_url = imported_articles.get(0).unwrap().url.to_string();
    let saved_date = imported_articles.get(0).unwrap().saved_date.to_string();
    save_url(arguments.key, article_url, saved_date, true).await
        .unwrap_or_else(|error| {
            eprintln!("Error has occurred during the saving of URLs into Omnivore:\n{}", error);
            exit(1);
        });

    println!("Successfully imported csv into Omnivore");
    Ok(())
}
