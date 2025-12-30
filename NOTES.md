target/debug/ror --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 --prove --output kek

time target/debug/ror --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 --prove --output kek
Generating ZK proof... (this may take a while)
Error: Custom("invalid value: integer `256`, expected u8")
target/debug/ror --private-key  --prove --output kek  103759.19s user 35.13s system 1511% cpu 1:54:25.92 total



./target/release/ror \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove-groth16 \
  --output test_groth16.png 2>&1


  # Basic image generation
cargo run --bin ror -- --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 --output image.png

# With Noir proof
cargo run --bin ror -- --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 --prove --output image.png

# Integration test
cargo run --bin test_noir_integration
