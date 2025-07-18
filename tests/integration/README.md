# Backward Compatibility and End-to-End Testing

This directory contains tests to ensure that the Walker tool maintains backward compatibility with existing scripts and usage patterns, and that it works correctly with real-world projects.

## Backward Compatibility Tests

The `backward_compatibility_test.rs` file contains tests that verify:

1. **Default Behavior**: The Walker works with default settings (no command-line arguments)
2. **ParallelWalker Compatibility**: The ParallelWalker produces the same results as the regular Walker
3. **CLI Backward Compatibility**: The CLI argument parsing maintains backward compatibility
4. **Real-World Project Structure**: The Walker handles real-world project structures correctly
5. **Existing Script Compatibility**: The Walker works with existing scripts (simulated by direct API calls)
6. **Edge Cases**: The Walker handles edge cases correctly
7. **Permission Handling**: The Walker handles permission errors gracefully
8. **Symlink Handling**: The Walker handles symlinks correctly

## End-to-End Tests

The `end_to_end_test.rs` file contains tests that verify:

1. **Complex Monorepo**: The Walker works correctly with a complex monorepo structure
2. **Walker/ParallelWalker Equivalence**: The Walker and ParallelWalker produce identical results
3. **Large Project Performance**: The Walker handles large projects efficiently
4. **Complex Exports Handling**: The Walker handles projects with complex exports fields correctly
5. **Browser Field Handling**: The Walker handles projects with browser field correctly
6. **TypeScript Support Detection**: The Walker handles projects with TypeScript support correctly
7. **Module System Detection**: The Walker handles projects with different module systems correctly

## Regression Tests

The `regression_test.rs` file contains tests that verify:

1. **Edge Cases**: The Walker handles edge cases correctly
2. **Unusual Module Configs**: The Walker handles packages with unusual module configurations correctly
3. **Unusual Dependency Configs**: The Walker handles packages with unusual dependency configurations correctly
4. **Unusual Version Fields**: The Walker handles packages with unusual version fields correctly
5. **Unusual Name Fields**: The Walker handles packages with unusual name fields correctly
6. **Unusual Engines Fields**: The Walker handles packages with unusual engines fields correctly
7. **Unusual License Fields**: The Walker handles packages with unusual license fields correctly
8. **Unusual File Structures**: The Walker handles packages with unusual file structures correctly
9. **Unusual Exports Fields**: The Walker handles packages with unusual exports fields correctly

## Running the Tests

To run these tests, use the following command:

```bash
cargo test --test integration_tests
```

Note: Some tests may be skipped on certain platforms (e.g., symlink tests on Windows without admin privileges) or in CI environments (e.g., large project performance tests).
