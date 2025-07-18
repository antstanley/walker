use std::time::Instant;
use tempfile::tempdir;
use walker::{
    core::{walker::Walker, parallel_walker::ParallelWalker},
    error::Result,
    models::config::Settings,
};

// Import the test fixture generator
use crate::fixtures::generate_large_project::generate_large_project;

/// Benchmark test for comparing different optimization strategies
#[test]
#[ignore] // Ignore by default as it's a benchmark test
fn benchmark_optimization_strategies() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Generate a medium-sized project (20 packages at each level, 4 levels deep)
    let count = generate_large_project(temp_dir.path(), 20, 4)?;
    println!("Generated {} packages for benchmark test", count);
    
    // Base settings
    let mut base_settings = Settings::default();
    base_settings.scan_path = temp_dir.path().to_path_buf();
    
    // Test 1: Sequential, with size calculation
    let mut settings = base_settings.clone();
    settings.parallel = false;
    settings.calculate_size = true;
    
    let walker = Walker::new(settings);
    let start = Instant::now();
    let results = walker.find_packages()?;
    let sequential_with_size_duration = start.elapsed();
    
    println!("Sequential with size: Found {} packages in {:?}", 
        results.len(), sequential_with_size_duration);
    
    // Test 2: Sequential, without size calculation
    let mut settings = base_settings.clone();
    settings.parallel = false;
    settings.calculate_size = false;
    
    let walker = Walker::new(settings);
    let start = Instant::now();
    let results = walker.find_packages()?;
    let sequential_without_size_duration = start.elapsed();
    
    println!("Sequential without size: Found {} packages in {:?}", 
        results.len(), sequential_without_size_duration);
    
    // Test 3: Parallel, with size calculation
    let mut settings = base_settings.clone();
    settings.parallel = true;
    settings.calculate_size = true;
    
    let walker = ParallelWalker::new(settings);
    let start = Instant::now();
    let results = walker.analyze()?;
    let parallel_with_size_duration = start.elapsed();
    
    println!("Parallel with size: Found {} packages in {:?}", 
        results.packages.len(), parallel_with_size_duration);
    
    // Test 4: Parallel, without size calculation
    let mut settings = base_settings.clone();
    settings.parallel = true;
    settings.calculate_size = false;
    
    let walker = ParallelWalker::new(settings);
    let start = Instant::now();
    let results = walker.analyze()?;
    let parallel_without_size_duration = start.elapsed();
    
    println!("Parallel without size: Found {} packages in {:?}", 
        results.packages.len(), parallel_without_size_duration);
    
    // Test 5: Parallel with streaming results
    let mut settings = base_settings.clone();
    settings.parallel = true;
    settings.calculate_size = false;
    settings.stream_results = true; // New setting for streaming
    
    let walker = ParallelWalker::new(settings);
    let start = Instant::now();
    let results = walker.analyze()?;
    let parallel_streaming_duration = start.elapsed();
    
    println!("Parallel with streaming: Found {} packages in {:?}", 
        results.packages.len(), parallel_streaming_duration);
    
    // Print comparison summary
    println!("\nPerformance Comparison:");
    println!("1. Sequential with size:    {:?}", sequential_with_size_duration);
    println!("2. Sequential without size: {:?}", sequential_without_size_duration);
    println!("3. Parallel with size:      {:?}", parallel_with_size_duration);
    println!("4. Parallel without size:   {:?}", parallel_without_size_duration);
    println!("5. Parallel with streaming: {:?}", parallel_streaming_duration);
    
    // Calculate improvements
    let base_time = sequential_with_size_duration.as_secs_f64();
    
    println!("\nImprovement Factors (higher is better):");
    println!("Sequential without size: {:.2}x", 
        base_time / sequential_without_size_duration.as_secs_f64());
    println!("Parallel with size:      {:.2}x", 
        base_time / parallel_with_size_duration.as_secs_f64());
    println!("Parallel without size:   {:.2}x", 
        base_time / parallel_without_size_duration.as_secs_f64());
    println!("Parallel with streaming: {:.2}x", 
        base_time / parallel_streaming_duration.as_secs_f64());
    
    Ok(())
}

/// Memory usage benchmark
#[test]
#[ignore] // Ignore by default as it's a benchmark test
fn benchmark_memory_usage() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Generate a large project (30 packages at each level, 5 levels deep)
    let count = generate_large_project(temp_dir.path(), 30, 5)?;
    println!("Generated {} packages for memory benchmark", count);
    
    // Base settings
    let mut base_settings = Settings::default();
    base_settings.scan_path = temp_dir.path().to_path_buf();
    
    // Test with different batch sizes for streaming
    let batch_sizes = [10, 50, 100, 500, 1000];
    
    for batch_size in batch_sizes {
        let mut settings = base_settings.clone();
        settings.parallel = true;
        settings.calculate_size = false;
        settings.stream_results = true;
        settings.batch_size = batch_size;
        
        let walker = ParallelWalker::new(settings);
        let start = Instant::now();
        let results = walker.analyze()?;
        let duration = start.elapsed();
        
        println!("Batch size {}: Found {} packages in {:?}", 
            batch_size, results.packages.len(), duration);
    }
    
    Ok(())
}