for file in tests/*.rs; do
    cargo test --test "$(basename "$file" .rs)"
done