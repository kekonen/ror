# Solana Development Setup

## Prerequisites Installation

### 1. Install Solana CLI

```bash
# Install Solana CLI (v1.18+)
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Add to PATH (add to ~/.zshrc if not automatic)
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Verify installation
solana --version

# Configure for local development
solana config set --url localhost
```

### 2. Install Anchor Framework

```bash
# Install Anchor Version Manager (avm)
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force

# Install Anchor 0.30.1 (stable version compatible with Solana 1.18)
avm install 0.30.1
avm use 0.30.1

# Verify installation
anchor --version
```

### 3. Install Node.js (for testing)

```bash
# Check if Node.js installed
node --version  # Should be v18+

# If not installed:
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs
```

## Project Structure

```
ror/
├── solana-program/          # New Solana program directory
│   ├── Anchor.toml         # Anchor configuration
│   ├── Cargo.toml          # Rust workspace
│   ├── package.json        # Node dependencies for tests
│   ├── programs/
│   │   └── rorschach/      # The Solana program
│   │       ├── Cargo.toml
│   │       └── src/
│   │           └── lib.rs  # Program code
│   ├── tests/              # TypeScript/JavaScript tests
│   │   └── rorschach.ts
│   └── keys/               # Program keypair (gitignored)
└── ... (existing files)
```

## Quick Start

Once tools are installed:

```bash
# From project root
cd /home/daniil/ror

# Initialize Anchor project
anchor init solana-program --no-git
cd solana-program

# Build program
anchor build

# Start local validator (separate terminal)
solana-test-validator

# Run tests (in another terminal)
anchor test --skip-local-validator

# Deploy to devnet
solana config set --url devnet
anchor build
anchor deploy
```

## Configuration Files

### Anchor.toml
```toml
[toolchain]
anchor_version = "0.30.1"

[features]
resolution = true
skip-lint = false

[programs.localnet]
rorschach = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"

[programs.devnet]
rorschach = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "Localnet"
wallet = "~/.config/solana/id.json"

[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
```

## Next Steps After Installation

1. Run the setup commands above
2. I'll generate the actual Solana program code
3. We'll implement Groth16 verification using native syscall
4. Add NFT minting logic
5. Test locally with solana-test-validator

## Why This Works

- **Native Groth16**: Solana runtime has built-in `groth16::verify()` syscall
- **No Barretenberg issues**: Solana doesn't care which tool generated the proof
- **Your existing circuit works**: Just need to submit proof to Solana instead of Ethereum
- **Local proving**: Generate proofs on your machine, submit to Solana
- **Production ready**: This is how real Solana ZK projects work

## Cost Comparison

| Operation | Ethereum | Solana |
|-----------|----------|--------|
| Deploy verifier | ~$50-100 | ~$10 (0.1 SOL) |
| Verify proof | ~$50-100 | ~$0.0005 (0.000005 SOL) |
| Mint NFT | ~$20-50 | ~$0.0001 (0.000001 SOL) |

---

**Status**: Awaiting Solana CLI and Anchor installation
**Next**: Generate Solana program code once tools are ready
