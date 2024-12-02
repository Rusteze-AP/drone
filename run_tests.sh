#!/bin/bash

# Find all `.rs` files directly inside the `tests` folder (exclude subfolders)
test_files=$(find tests -maxdepth 1 -name '*.rs' -exec basename {} .rs \; | tr '\n' ' ')

# If no test files are found, exit
if [ -z "$test_files" ]; then
    echo "No test files found directly in the 'tests' folder."
    exit 1
fi

# Run `cargo test` with the test files
echo "Running tests for: $test_files"
cargo test $(printf -- "--test %s " $test_files)
