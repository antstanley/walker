//! Command-line argument configuration source

use std::path::PathBuf;

use crate::cli::args::{Args, OutputFormat as CliOutputFormat};
use crate::error::Result;
use crate::models::config::{OutputFormat, PartialSettings};
use super::ConfigSource;

/// Command-line argument configuration source
#[derive(Debug)]
pub struct CliConfig {
    args: CliArgs,
    name: String,
    priority: u8,
}

/// Command-line arguments structure
#[derive(Debug, Clone, Default)]
pub struct CliArgs {
    pub path: Option<PathBuf>,
    pub exclude: Option<Vec<String>>,
    pub max_depth: Option<usize>,
    pub output_format: Option<OutputFormat>,
    pub output_file: Option<PathBuf>,
    pub no_size: bool,
    pub no_parallel: bool,
    pub no_cache: bool,
    pub quiet: bool,
    pub verbose: bool,
    pub follow_links: bool,
    pub no_dev_deps: bool,
    pub no_peer_deps: bool,
    pub no_optional_deps: bool,
    pub no_colors: bool,
    pub cache_dir: Option<PathBuf>,
    pub no_progress: bool,
    pub stream_results: bool,
    pub batch_size: Option<usize>,
    pub memory_limit: Option<usize>,
    pub config: Option<PathBuf>,
}

impl CliConfig {
    /// Create a new CLI configuration source
    pub fn new(args: CliArgs) -> Self {
        Self {
            args,
            name: "command-line arguments".to_string(),
            priority: 30, // Highest priority
        }
    }

    /// Create a CLI configuration source from Args
    pub fn from_args(args: &Args) -> Self {
        let cli_args = CliArgs {
            path: args.path.clone(),
            exclude: if args.exclude.is_empty() { None } else { Some(args.exclude.clone()) },
            max_depth: args.max_depth,
            output_format: Some(match args.output {
                CliOutputFormat::Text => OutputFormat::Text,
                CliOutputFormat::Json => OutputFormat::Json,
                CliOutputFormat::Csv => OutputFormat::Csv,
            }),
            output_file: args.output_file.clone(),
            no_size: args.no_size,
            no_parallel: args.no_parallel,
            no_cache: args.no_cache,
            quiet: args.quiet,
            verbose: args.verbose,
            follow_links: args.follow_links,
            no_dev_deps: args.no_dev_deps,
            no_peer_deps: args.no_peer_deps,
            no_optional_deps: args.no_optional_deps,
            no_colors: args.no_colors,
            cache_dir: args.cache_dir.clone(),
            no_progress: args.no_progress,
            stream_results: args.stream_results,
            batch_size: args.batch_size,
            memory_limit: args.memory_limit,
            config: args.config.clone(),
        };

        Self::new(cli_args)
    }

    /// Set the priority for this configuration source
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Get the config file path if specified
    pub fn config_path(&self) -> Option<&PathBuf> {
        self.args.config.as_ref()
    }
}

impl ConfigSource for CliConfig {
    fn load(&self) -> Result<PartialSettings> {
        let mut settings = PartialSettings::default();

        // Convert CLI args to PartialSettings
        if let Some(path) = &self.args.path {
            settings.scan_path = Some(path.clone());
        }

        if let Some(exclude) = &self.args.exclude {
            settings.exclude_patterns = Some(exclude.clone());
        }

        if let Some(max_depth) = self.args.max_depth {
            settings.max_depth = Some(max_depth);
        }

        if let Some(format) = &self.args.output_format {
            settings.output_format = Some(format.clone());
        }

        if let Some(output_file) = &self.args.output_file {
            settings.output_file = Some(output_file.clone());
        }

        // Boolean flags
        if self.args.no_size {
            settings.calculate_size = Some(false);
        }

        if self.args.no_parallel {
            settings.parallel = Some(false);
        }

        if self.args.no_cache {
            settings.cache_enabled = Some(false);
        }

        if self.args.quiet {
            settings.quiet = Some(true);
        }

        if self.args.verbose {
            settings.verbose = Some(true);
        }

        if self.args.follow_links {
            settings.follow_links = Some(true);
        }

        if self.args.no_dev_deps {
            settings.include_dev_deps = Some(false);
        }

        if self.args.no_peer_deps {
            settings.include_peer_deps = Some(false);
        }

        if self.args.no_optional_deps {
            settings.include_optional_deps = Some(false);
        }

        if self.args.no_colors {
            settings.use_colors = Some(false);
        }

        if let Some(cache_dir) = &self.args.cache_dir {
            settings.cache_dir = Some(cache_dir.clone());
        }

        if self.args.no_progress {
            settings.show_progress = Some(false);
        }

        // New streaming options
        if self.args.stream_results {
            settings.stream_results = Some(true);
        }

        if let Some(batch_size) = self.args.batch_size {
            settings.batch_size = Some(batch_size);
        }

        if let Some(memory_limit) = self.args.memory_limit {
            settings.memory_limit_mb = Some(memory_limit);
        }

        Ok(settings)
    }

    fn is_available(&self) -> bool {
        // CLI args are always available
        true
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u8 {
        self.priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_config_source() {
        let args = CliArgs {
            path: Some(PathBuf::from("/cli/path")),
            exclude: Some(vec!["cli_exclude".to_string()]),
            max_depth: Some(10),
            output_format: Some(OutputFormat::Json),
            no_size: true,
            verbose: true,
            ..Default::default()
        };

        let cli_config = CliConfig::new(args);
        assert!(cli_config.is_available());
        assert_eq!(cli_config.priority(), 30);

        let settings = cli_config.load().unwrap();

        assert_eq!(settings.scan_path, Some(PathBuf::from("/cli/path")));
        assert_eq!(settings.exclude_patterns, Some(vec!["cli_exclude".to_string()]));
        assert_eq!(settings.max_depth, Some(10));
        assert!(matches!(settings.output_format, Some(OutputFormat::Json)));
        assert_eq!(settings.calculate_size, Some(false));
        assert_eq!(settings.verbose, Some(true));
    }

    #[test]
    fn test_from_args() {
        let cli_args = Args {
            path: Some(PathBuf::from("/test/path")),
            exclude: vec!["node_modules".to_string(), "dist".to_string()],
            max_depth: Some(5),
            output: CliOutputFormat::Json,
            output_file: Some(PathBuf::from("output.json")),
            quiet: true,
            verbose: false,
            no_size: true,
            config: None,
            no_parallel: true,
            no_cache: false,
            follow_links: true,
            no_dev_deps: true,
            no_peer_deps: false,
            no_optional_deps: true,
            no_colors: false,
            cache_dir: Some(PathBuf::from("/cache")),
            no_progress: true,
            stream_results: true,
            batch_size: Some(200),
            memory_limit: Some(1024),
            init: false
        };

        let cli_config = CliConfig::from_args(&cli_args);
        let settings = cli_config.load().unwrap();

        assert_eq!(settings.scan_path, Some(PathBuf::from("/test/path")));
        assert_eq!(settings.exclude_patterns, Some(vec!["node_modules".to_string(), "dist".to_string()]));
        assert_eq!(settings.max_depth, Some(5));
        assert!(matches!(settings.output_format, Some(OutputFormat::Json)));
        assert_eq!(settings.output_file, Some(PathBuf::from("output.json")));
        assert_eq!(settings.quiet, Some(true));
        assert_eq!(settings.verbose, Some(false));
        assert_eq!(settings.calculate_size, Some(false));
        assert_eq!(settings.parallel, Some(false));
        assert_eq!(settings.follow_links, Some(true));
        assert_eq!(settings.include_dev_deps, Some(false));
        assert_eq!(settings.include_optional_deps, Some(false));
        assert_eq!(settings.cache_dir, Some(PathBuf::from("/cache")));
        assert_eq!(settings.show_progress, Some(false));
        assert_eq!(settings.stream_results, Some(true));
        assert_eq!(settings.batch_size, Some(200));
        assert_eq!(settings.memory_limit_mb, Some(1024));
    }
}
