//! Integration test for the parallel walker

#[cfg(test)]
mod tests {
    use crate::core::{ParallelWalker, Walker};
    use crate::models::config::Settings;
    use crate::output::ProgressReporter;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn test_parallel_walker() {
        // Create test settings
        let mut settings = Settings::default();
        settings.scan_path = PathBuf::from(".");
        settings.parallel = true;
        
        // Create a parallel walker
        let walker = ParallelWalker::new(settings.clone());
        
        // Run analysis
        let results = walker.analyze().expect("Analysis should succeed");
        
        // Create a regular walker for comparison
        let regular_walker = Walker::new(settings);
        let regular_results = regular_walker.analyze().expect("Regular analysis should succeed");
        
        // Both should find the same number of packages
        assert_eq!(results.packages.len(), regular_results.packages.len());
    }
    
    #[test]
    fn test_parallel_walker_with_progress() {
        // Create test settings
        let mut settings = Settings::default();
        settings.scan_path = PathBuf::from(".");
        settings.parallel = true;
        
        // Create a parallel walker
        let walker = ParallelWalker::new(settings);
        
        // Create a progress reporter
        let reporter = Arc::new(ProgressReporter::new(false, true));
        let progress_callback = crate::output::create_progress_callback(reporter.clone());
        
        // Run analysis with progress reporting
        let results = walker.analyze_with_progress(progress_callback).expect("Analysis should succeed");
        
        // Should find at least one package (the current project)
        assert!(results.packages.len() > 0);
    }
}