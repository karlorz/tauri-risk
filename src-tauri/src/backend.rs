// src-tauri/src/backend.rs

use tauri::{AppHandle, Manager}; // Import Manager trait and AppHandle // Import BaseDirectory enum

use csv::ReaderBuilder;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::path::Path;

use risk_normalization_lib;

const CSV_FILE: &str = include_str!("../resources/generated_normal_trades.csv");

/// Reads trades from a CSV string using Serde deserialization.
fn read_trades_from_str(data: &str) -> Result<Vec<f64>, Box<dyn Error>> {
    // Initialize the CSV reader with headers.
    let mut rdr = ReaderBuilder::new()
        .has_headers(true) // Set to false if your CSV doesn't have headers.
        .from_reader(data.as_bytes());

    let mut trades = Vec::new();

    // Iterate over each record and deserialize into `TradeRecord`.
    for result in rdr.deserialize() {
        let record: TradeRecord = result?; // Deserialize the record.
        trades.push(record.value); // Extract the `value` field.
    }

    Ok(trades)
}

/// Struct to hold the serialized results
#[derive(Serialize)]
pub struct RiskNormalizationResultSerializable {
    pub safe_f_mean: f64,
    pub safe_f_stdev: f64,
    pub car25_mean: f64,
    pub car25_stdev: f64,
}

#[derive(Debug, Deserialize)]
struct TradeRecord {
    #[serde(rename = "value")] // Adjust the field name based on your CSV header
    value: f64,
}

/// Reads trades from a CSV file using Serde deserialization.
fn read_trades_from_csv(path: &Path) -> Result<Vec<f64>, Box<dyn Error>> {
    // Open the CSV file.
    let file = File::open(path)?;

    // Initialize the CSV reader with headers.
    let mut rdr = ReaderBuilder::new()
        .has_headers(true) // Set to false if your CSV doesn't have headers.
        .from_reader(file);

    let mut trades = Vec::new();

    // Iterate over each record and deserialize into `TradeRecord`.
    for result in rdr.deserialize() {
        let record: TradeRecord = result?; // Deserialize the record.
        trades.push(record.value); // Extract the `value` field.
    }

    Ok(trades)
}


/// Tauri command to perform risk normalization
#[tauri::command]
pub fn risk_normalization_command(
    handle: AppHandle,
) -> Result<RiskNormalizationResultSerializable, String> {
    // Parse trades from the embedded CSV data
    let trades = read_trades_from_str(CSV_FILE).map_err(|e| e.to_string())?;

    // Check if trades are empty
    if trades.is_empty() {
        return Err("No trades data found.".into());
    }

    // Set parameters
    let number_of_years_in_csv = 28.0;
    let average_trades_per_year = trades.len() as f64 / number_of_years_in_csv;
    let years_to_forecast = 2.0;
    let number_days_in_forecast = (years_to_forecast * 252.0) as usize;
    let number_trades_in_forecast = (average_trades_per_year * years_to_forecast) as usize;
    let initial_capital = 100000.0;
    let tail_percentile = 5.0;
    let drawdown_tolerance = 0.10;
    let number_equity_in_cdf = 10000;
    let number_repetitions = 5;

    // Initialize RNG
    let seed: Option<u64> = None; // keep line for record
    // let seed: Option<u64> = Some(42); // keep line for record
    let mut rng = match seed {
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
        None => StdRng::from_entropy(),
    };

    // Call the risk_normalization function
    let result: risk_normalization_lib::RiskNormalizationResult = risk_normalization_lib::risk_normalization_concurrent(
        &trades,
        number_days_in_forecast,
        number_trades_in_forecast,
        initial_capital,
        tail_percentile,
        drawdown_tolerance,
        number_equity_in_cdf,
        number_repetitions,
        &mut rng,
    )
    .map_err(|e: risk_normalization_lib::RiskNormalizationError| e.to_string())?;

    // Return the result
    Ok(RiskNormalizationResultSerializable {
        safe_f_mean: result.safe_f_mean,
        safe_f_stdev: result.safe_f_stdev,
        car25_mean: result.car25_mean,
        car25_stdev: result.car25_stdev,
    })
}
