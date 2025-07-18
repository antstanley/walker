# Walker

A robust Node.js package analyzer for module system support detection.

## Overview

Walker is a command-line tool that scans directory structures to identify and analyze JavaScript/Node.js packages. It provides detailed information about each package's module system support (ESM/CommonJS), TypeScript support, browser compatibility, and dependency analysis.

The tool is designed to help developers understand their project's dependency ecosystem, plan migrations (e.g., from CommonJS to ESM), and analyze package characteristics across large codebases.

## Features

- **Module System Analysis**: Detect ESM, CommonJS, and dual-mode packages
- **Enhanced Package Information**: TypeScript support, browser compatibility, Node.js version requirements
- **Dependency Analysis**: Count and categorize dependencies (production, dev, peer, optional)
- **Performance Optimized**: Parallel processing, caching, and configurable performance options
- **Multiple Output Formats**: Human-readable text, JSON for programmatic use, CSV for spreadsheets
- **Robust Error Handling**: Graceful recovery from permission issues and corrupted files
- **Configurable**: Command-line options and configuration file support
- **Progress Reporting**: Real-time feedback during analysis

## Installation

### From Cargo (Recommended)

```bash
cargo install walker
```

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/walker.git
cd walker

# Build and install
cargo build --release
cargo install --path .
```

## Quick Start

```bash
# Analyze the current directory
walker

# Analyze a specific directory
walker --path ./my-project

# Generate JSON output
walker --path ./my-project --output json

# Save results to a file
walker --path ./my-project --output json --output-file results.json
```

## Usage

```
USAGE:
    walker [OPTIONS]

OPTIONS:
    -p, --path <PATH>              Directory to scan for Node.js packages (defaults to current directory)
    -e, --exclude <PATTERN>        Glob patterns for directories to exclude (can be specified multiple times)
    --max-depth <DEPTH>            Maximum directory depth to traverse
    -o, --output <FORMAT>          Output format: text, json, or csv [default: text]
    --output-file <FILE>           File to write output to (uses stdout if not specified)
    -q, --quiet                    Suppress non-essential output
    -v, --verbose                  Show detailed progress and debug information
    --no-size                      Skip package size calculation for faster scanning
    -c, --config <FILE>            Path to configuration file
    --no-parallel                  Disable parallel processing
    --no-cache                     Disable result caching
    --follow-links                 Follow symbolic links during directory traversal
    --no-dev-deps                  Exclude development dependencies from analysis
    --no-peer-deps                 Exclude peer dependencies from analysis
    --no-optional-deps             Exclude optional dependencies from analysis
    --no-colors                    Disable colored output
    --cache-dir <DIR>              Custom cache directory path
    --no-progress                  Disable progress bars
    --init                         Create a default configuration file
    -h, --help                     Print help information
    -V, --version                  Print version information
```

## Common Workflows

### Quick Analysis

For a fast scan of a project:

```bash
walker --path ./my-project --no-size --exclude node_modules
```

### Detailed Analysis

For comprehensive information about all packages:

```bash
walker --path ./my-project --verbose
```

### Generate Reports

For spreadsheet analysis:

```bash
walker --path ./my-project --output csv --output-file report.csv
```

### Production Dependencies Only

To focus only on production dependencies:

```bash
walker --path ./my-project --no-dev-deps --no-peer-deps --no-optional-deps
```

## Configuration File

Walker supports configuration via a TOML file. You can create a default configuration file with:

```bash
walker --init
```

This creates a `.walker.toml` file in the current directory with commented examples of all available options.

Example configuration:

```toml
# Directory to scan for packages
scan_path = "./my-project"

# Patterns for directories to exclude from scanning
exclude_patterns = [
    "node_modules",
    ".git",
    "target",
    "dist",
    "build"
]

# Maximum directory depth to traverse
max_depth = 10

# Output format: "text", "json", or "csv"
output_format = "text"

# Whether to calculate package sizes
calculate_size = true
```

Walker looks for configuration files in the following locations (in order):

1. Path specified with `--config`
2. `.walker.toml` in the current directory
3. `~/.walker.toml` in the user's home directory
4. `~/.config/walker/config.toml` (XDG config directory)

Command-line options take precedence over configuration file settings.

## Output Formats

### Text (Default)

Human-readable colored output with package details and summary statistics.

### JSON

Structured JSON output suitable for programmatic processing:

```json
{
  "packages": [
    {
      "path": "/path/to/package",
      "name": "package-name",
      "version": "1.0.0",
      "module_support": {
        "esm": true,
        "cjs": true
      },
      "typescript_support": true,
      "browser_support": false,
      "dependencies": {
        "production_count": 5,
        "development_count": 10,
        "peer_count": 2,
        "optional_count": 0
      }
    }
  ],
  "summary": {
    "total_packages": 1,
    "esm_supported": 1,
    "cjs_supported": 1,
    "typescript_supported": 1,
    "browser_supported": 0
  }
}
```

### CSV

Comma-separated values format for spreadsheet analysis:

```
path,name,version,esm_support,cjs_support,typescript_support,browser_support,production_deps,dev_deps,peer_deps,optional_deps
/path/to/package,package-name,1.0.0,true,true,true,false,5,10,2,0
```

## Error Handling

Walker is designed to handle errors gracefully:

- **Permission Denied**: Logs a warning and continues with accessible directories
- **Corrupted JSON**: Reports the error and continues with other packages
- **Inaccessible Directories**: Skips with a warning and continues
- **Critical Errors**: Exits with a non-zero status code and clear error message

## Performance Considerations

For large codebases:

- Use `--no-size` to skip package size calculation (significant performance boost)
- Use `--exclude` to skip irrelevant directories
- Consider using `--max-depth` to limit directory traversal depth
- The tool uses parallel processing by default; use `--no-parallel` on systems with limited resources

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
