use std::time::Instant;
use tempfile::tempdir;
use walker::{
    core::walker::Walker,
    error::Result,
    models::config::Settings,
};

// Import the test fixture generator
use crate::fixtures::generate_large_project::generate_large_project;

#[test]
#[ignore] // Ignore by default as it's a performance test that takes time
fn test_walker_performance_small() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Generate a small project (10 packages at each level, 3 levels deep)
    let count = generate_large_project(temp_dir.path(), 10, 3)?;
    println!("Generated {} packages for performance test", count);
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Measure time to find packages
    let start = Instant::now();
    let packages = walker.find_packages()?;
    let duration = start.elapsed();
    
    println!("Found {} packages in {:?}", packages.len(), duration);
    
    // Basic performance assertion - should be reasonably fast
    // This is just a sanity check, not a strict performance requirement
    assert!(duration.as_secs() < 5, "Performance test took too long: {:?}", duration);
    
    Ok(())
}

#[test]
#[ignore] // Ignore by default as it's a performance test that takes time
fn test_walker_performance_medium() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Generate a medium project (20 packages at each level, 4 levels deep)
    let count = generate_large_project(temp_dir.path(), 20, 4)?;
    println!("Generated {} packages for performance test", count);
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Measure time to find packages
    let start = Instant::now();
    let packages = walker.find_packages()?;
    let duration = start.elapsed();
    
    println!("Found {} packages in {:?}", packages.len(), duration);
    
    // Basic performance assertion - should be reasonably fast
    // This is just a sanity check, not a strict performance requirement
    assert!(duration.as_secs() < 10, "Performance test took too long: {:?}", duration);
    
    Ok(())
}

#[test]
#[ignore] // Ignore by default as it's a very large performance test
fn test_walker_performance_large() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Generate a large project (30 packages at each level, 5 levels deep)
    let count = generate_large_project(temp_dir.path(), 30, 5)?;
    println!("Generated {} packages for performance test", count);
    
    // Create settings with parallel processing
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    settings.parallel = true;
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Measure time to find packages
    let start = Instant::now();
    let packages = walker.find_packages()?;
    let duration = start.elapsed();
    
    println!("Found {} packages in {:?}", packages.len(), duration);
    
    // Basic performance assertion - should be reasonably fast
    // This is just a sanity check, not a strict performance requirement
    assert!(duration.as_secs() < 30, "Performance test took too long: {:?}", duration);
    
    Ok(())
}

#[test]
fn test_walker_parallel_vs_sequential() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Generate a medium project (15 packages at each level, 3 levels deep)
    let count = generate_large_project(temp_dir.path(), 15, 3)?;
    println!("Generated {} packages for comparison test", count);
    
    // Test sequential processing
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    settings.parallel = false;
    
    let walker = Walker::new(settings.clone());
    
    let start = Instant::now();
    let sequential_packages = walker.find_packages()?;
    let sequential_duration = start.elapsed();
    
    println!("Sequential: Found {} packages in {:?}", sequential_packages.len(), sequential_duration);
    
    // Test parallel processing
    settings.parallel = true;
    
    let walker = Walker::new(settings);
    
    let start = Instant::now();
    let parallel_packages = walker.find_packages()?;
    let parallel_duration = start.elapsed();
    
    println!("Parallel: Found {} packages in {:?}", parallel_packages.len(), parallel_duration);
    
    // Both should find the same number of packages
    assert_eq!(sequential_packages.len(), parallel_packages.len());
    
    // Parallel should generally be faster, but this is not guaranteed
    // on all systems, so we don't make a strict assertion here
    println!("Speed improvement: {:.2}x", sequential_duration.as_secs_f64() / parallel_duration.as_secs_f64());
    
    Ok(())
}