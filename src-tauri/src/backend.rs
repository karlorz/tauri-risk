// src-tauri/src/backend.rs

use csv::ReaderBuilder;
use rand::distributions::{Distribution, Uniform};
use rand::SeedableRng;
use rand::rngs::StdRng;
use statrs::statistics::Statistics;
use serde::Serialize;
use std::error::Error;
use std::fmt;
use std::path::Path;
use tauri::command;
use tauri::{AppHandle, Manager}; 
use tauri::path::BaseDirectory; 

/// Struct to hold the serialized results
#[derive(Serialize)]
pub struct RiskNormalizationResultSerializable {
    pub safe_f_mean: f64,
    pub safe_f_stdev: f64,
    pub car25_mean: f64,
    pub car25_stdev: f64,
}

/// Struct to hold the results
#[derive(Debug)]
pub struct RiskNormalizationResult {
    pub safe_f_mean: f64,
    pub safe_f_stdev: f64,
    pub car25_mean: f64,
    pub car25_stdev: f64,
}

/// Function to read trades from a CSV file
fn read_trades_from_csv<P: AsRef<Path>>(filename: P) -> Result<Vec<f64>, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new().has_headers(false).from_path(filename)?;
    let mut trades = Vec::new();
    for result in rdr.records() {
        let record = result?;
        for field in record.iter() {
            if let Ok(value) = field.parse::<f64>() {
                trades.push(value);
            }
        }
    }
    Ok(trades)
}

/// Function to compute mean of a slice
fn compute_mean(data: &[f64]) -> f64 {
    data.mean()
}

/// Function to compute standard deviation of a slice
fn compute_std_dev(data: &[f64], mean: f64) -> f64 {
    let variance = data.iter().map(|value| {
        let diff = value - mean;
        diff * diff
    }).sum::<f64>() / data.len() as f64;
    variance.sqrt()
}

/// Function to calculate maximum drawdown from equity curve
fn calculate_drawdown(equity_curve: &[f64]) -> f64 {
    let mut peak = equity_curve[0];
    let mut max_drawdown = 0.0;
    for &equity in equity_curve.iter().skip(1) {
        if equity > peak {
            peak = equity;
        }
        let drawdown = (peak - equity) / peak;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }
    max_drawdown
}

/// Function to calculate CAGR
fn calculate_cagr(initial_equity: f64, final_equity: f64, years: f64) -> f64 {
    if initial_equity <= 0.0 || final_equity <= 0.0 || years <= 0.0 {
        return 0.0;
    }
    ((final_equity / initial_equity).powf(1.0 / years) - 1.0) * 100.0
}

/// Function to simulate one equity sequence and calculate max drawdown
fn make_one_equity_sequence(
    trades: &[f64],
    fraction: f64,
    number_trades_in_forecast: usize,
    initial_capital: f64,
    rng: &mut StdRng,
) -> (Vec<f64>, f64) {
    let mut equity_curve = vec![initial_capital];
    let trade_dist = Uniform::from(0..trades.len());
    for _ in 0..number_trades_in_forecast {
        let idx = trade_dist.sample(rng);
        let trade_return = trades[idx] * fraction * equity_curve.last().unwrap();
        let new_equity = equity_curve.last().unwrap() + trade_return;
        equity_curve.push(new_equity);
    }
    let max_drawdown = calculate_drawdown(&equity_curve);
    (equity_curve, max_drawdown)
}

/// Function to analyze distribution of drawdowns and compute tail risk
fn analyze_distribution_of_drawdown(
    trades: &[f64],
    fraction: f64,
    number_trades_in_forecast: usize,
    initial_capital: f64,
    drawdown_tolerance: f64,
    number_equity_in_cdf: usize,
    rng: &mut StdRng,
) -> f64 {
    let mut count_exceed = 0;
    for _ in 0..number_equity_in_cdf {
        let (_equity_curve, max_drawdown) = make_one_equity_sequence(
            trades,
            fraction,
            number_trades_in_forecast,
            initial_capital,
            rng,
        );
        if max_drawdown > drawdown_tolerance {
            count_exceed += 1;
        }
    }
    count_exceed as f64 / number_equity_in_cdf as f64
}

/// Function to compute statistics
fn compute_statistics(data: &[f64]) -> (f64, f64) {
    let mean = compute_mean(data);
    let stdev = compute_std_dev(data, mean);
    (mean, stdev)
}

/// Module Error for better error messages
#[derive(Debug)]
struct RiskNormalizationError(String);

impl fmt::Display for RiskNormalizationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RiskNormalizationError: {}", self.0)
    }
}

impl Error for RiskNormalizationError {}

/// risk_normalization function implementation
fn risk_normalization(
    trades: &[f64],
    number_days_in_forecast: usize,
    number_trades_in_forecast: usize,
    initial_capital: f64,
    tail_percentile: f64,
    drawdown_tolerance: f64,
    number_equity_in_cdf: usize,
    number_repetitions: usize,
    rng: &mut StdRng,
) -> Result<RiskNormalizationResult, Box<dyn Error>> {
    let desired_accuracy = 0.003;
    let mut safe_f_list = Vec::with_capacity(number_repetitions);
    let mut car25_list = Vec::with_capacity(number_repetitions);

    let tail_target = tail_percentile / 100.0;

    for rep in 0..number_repetitions {
        let mut fraction = 1.0;
        let tolerance = desired_accuracy;
        let max_iterations = 1000;
        let mut iteration = 0;
        let mut done = false;

        let mut lower_bound = 0.0;
        let mut upper_bound = 10.0; // Arbitrary upper limit for fraction
        let mut tail_risk = 0.0;

        while !done && iteration < max_iterations {
            fraction = (lower_bound + upper_bound) / 2.0;
            tail_risk = analyze_distribution_of_drawdown(
                trades,
                fraction,
                number_trades_in_forecast,
                initial_capital,
                drawdown_tolerance,
                number_equity_in_cdf,
                rng,
            );

            if (tail_risk - tail_target).abs() < tolerance {
                done = true;
            } else if tail_risk > tail_target {
                upper_bound = fraction;
            } else {
                lower_bound = fraction;
            }
            iteration += 1;
        }

        safe_f_list.push(fraction);

        // Simulate equity curves to collect CARs
        let mut car_list = Vec::with_capacity(number_equity_in_cdf);
        for _ in 0..number_equity_in_cdf {
            let (equity_curve, _max_drawdown) = make_one_equity_sequence(
                trades,
                fraction,
                number_trades_in_forecast,
                initial_capital,
                rng,
            );

            let years = number_days_in_forecast as f64 / 252.0;
            let cagr = calculate_cagr(initial_capital, *equity_curve.last().unwrap(), years);
            car_list.push(cagr);
        }

        // Calculate the 25th percentile CAR (CAR25)
        car_list.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let index = ((0.25 * car_list.len() as f64).ceil() as usize).saturating_sub(1);
        let car25 = car_list.get(index).ok_or_else(|| {
            RiskNormalizationError(format!(
                "Failed to compute CAR25 for repetition {}",
                rep + 1
            ))
        })?;
        car25_list.push(*car25);

        // Print Compound Annual Return for this repetition with high precision
        println!("Compound Annual Return: {:.5}%", car25);
    }

    // Compute statistics for safe_f
    let (safe_f_mean, safe_f_stdev) = compute_statistics(&safe_f_list);

    // Compute statistics for CAR25
    let (car25_mean, car25_stdev) = compute_statistics(&car25_list);

    Ok(RiskNormalizationResult {
        safe_f_mean,
        safe_f_stdev,
        car25_mean,
        car25_stdev,
    })
}

/// Tauri command to perform risk normalization
#[tauri::command]
pub fn risk_normalization_command(handle: AppHandle) -> Result<RiskNormalizationResultSerializable, String> {
    // Path to CSV within Tauri project
    // let path_to_trades = "./resources/generated_normal_trades.csv";
    let resource_path = handle.path().resolve("resources/generated_normal_trades.csv", BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;
    
    let path_to_trades = resource_path.as_path();
    // Read trades from CSV
    let trades = read_trades_from_csv(path_to_trades).map_err(|e| e.to_string())?;

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
    let seed: Option<u64> = None;
    let mut rng = match seed {
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
        None => StdRng::from_entropy(),
    };

    // Call the risk_normalization function
    let result = risk_normalization(
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
    .map_err(|e| e.to_string())?;

    // Return the result
    Ok(RiskNormalizationResultSerializable {
        safe_f_mean: result.safe_f_mean,
        safe_f_stdev: result.safe_f_stdev,
        car25_mean: result.car25_mean,
        car25_stdev: result.car25_stdev,
    })
}