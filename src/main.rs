use std::error::Error;
use std::process::exit;

use clap::Parser;

use crate::csv_utils::write_logs;
use crate::structs::{Arguments, ImportResult};

mod structs;
mod csv_utils;
mod omnivore_lib;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();

    println!("\n=============================================");
    println!("Readwise to Omnivore importer");
    println!("Using API key: {}", arguments.key);
    println!("Using file path: {}", arguments.file_path);
    let articles = csv_utils::get_imported_articles(&arguments.file_path)
        .unwrap_or_else(|error| {
            eprintln!("Errors occurred while parsing the CSV: {}\nExiting application", error);
            exit(1);
        });

    let results = omnivore_lib::save_urls(arguments.key, &articles).await;
    let (success_results, rest_results): (Vec<ImportResult>, Vec<ImportResult>) =  results.into_iter().partition(|result| result.successful);
    let (invalid_results, error_results): (Vec<ImportResult>, Vec<ImportResult>) = rest_results.into_iter().partition(|result| result.is_invalid_url);

    let invalid_count = invalid_results.len();
    let error_count = error_results.len();
    let success_count = success_results.len();
    let total_count = invalid_count + error_count + success_count;
    println!("\n=============================================");
    println!("Total processed articles: {}", total_count);
    println!("\tAmount of success articles: {}", success_count);
    println!("\tAmount of invalid articles: {}", invalid_count);
    println!("\tAmount of error articles: {}", error_count);

    write_logs(articles, invalid_results, error_results);
    Ok(())
}