# EVM On-Chain Verification Guide

This guide explains how to verify Rorschach image proofs on Ethereum (or any EVM chain).

## Overview

The system uses **risc0's Groth16 bridge** to make proofs EVM-compatible:

```
Private Key → [risc0 Guest] → STARK Proof → [Bonsai Service] → Groth16 Proof → [EVM Verifier]
                                  ↓                                    ↓
                            ~200KB-2MB                            ~500 bytes
                         (off-chain verify)                    (on-chain verify)
```

## Architecture

### 1. STARK Proof (Off-chain, what we have now)
- **Size**: 200KB - 2MB
- **Generation**: 30-60 seconds locally
- **Verification**: Instant in Rust
- **Cost**: Free (runs locally)
- **Use case**: Fast local verification, development

### 2. Groth16 Proof (On-chain compatible)
- **Size**: ~500 bytes
- **Generation**: 2-5 minutes via Bonsai
- **Verification**: ~200K gas on-chain
- **Cost**: Bonsai API credits + gas fees
- **Use case**: Production on-chain verification

## Setup

### Prerequisites

1. **Bonsai API Key** (for Groth16 conversion)
   ```bash
   # Sign up at https://bonsai.xyz
   export BONSAI_API_KEY="your_api_key_here"
   export BONSAI_API_URL="https://api.bonsai.xyz"
   ```

2. **Build with Bonsai support**
   ```bash
   cargo build --release --features bonsai
   ```

3. **Deploy Verifier Contract**
   - Deploy `RiscZeroGroth16Verifier.sol` (risc0's standard contract)
   - Deploy `RorschachVerifier.sol` with the verifier address

## Usage

### Step 1: Generate Groth16 Proof

```bash
# Generate proof with Bonsai (sends to cloud for Groth16 conversion)
cargo run --release --features bonsai -- \
  --private-key 0xYOUR_PRIVATE_KEY \
  --prove-groth16 \
  --output image.png

# Outputs:
#   - image.png (the generated image)
#   - image.proof (STARK proof, for local verification)
#   - image.groth16 (Groth16 proof seal, for on-chain)
#   - image.journal (public outputs, for on-chain)
```

### Step 2: Deploy Contracts

```solidity
// 1. Deploy risc0's Groth16 verifier (one-time, reusable)
RiscZeroGroth16Verifier verifier = new RiscZeroGroth16Verifier(...);

// 2. Deploy RorschachVerifier
RorschachVerifier rorschach = new RorschachVerifier(address(verifier));
```

### Step 3: Verify On-Chain

```javascript
// Load proof files
const seal = fs.readFileSync('image.groth16');
const journal = fs.readFileSync('image.journal');

// Call verifier contract
const tx = await rorschach.verifyImage(seal, journal);
await tx.wait();

console.log('Image verified on-chain! ✓');
```

## Use Cases

### 1. **NFT Minting with Proof-of-Ownership**

Mint an NFT only if you prove you know the private key:

```solidity
contract RorschachNFT is ERC721 {
    RorschachVerifier public verifier;

    function mintWithProof(bytes calldata seal, bytes calldata journal) external {
        // Verify proof
        verifier.verifyImage(seal, journal);

        // Decode to get address
        (address owner, , , bytes memory imageBytes) =
            abi.decode(journal, (address, uint64, uint64, bytes));

        require(owner == msg.sender, "Proof must be for your address");

        // Mint NFT with image data
        _safeMint(msg.sender, tokenId);
        _setTokenURI(tokenId, imageBytes);
    }
}
```

### 2. **Anonymous Credential System**

Prove you own a private key without revealing which one:

```solidity
contract AnonymousProof {
    mapping(bytes32 => bool) public usedProofs;

    function proveOwnership(bytes calldata seal, bytes calldata journal) external {
        verifier.verifyImage(seal, journal);

        // Extract image hash as a unique identifier
        (, , , bytes memory imageBytes) =
            abi.decode(journal, (address, uint64, uint64, bytes));
        bytes32 imageHash = keccak256(imageBytes);

        require(!usedProofs[imageHash], "Proof already used");
        usedProofs[imageHash] = true;

        // Grant credential/access
        grantAccess(msg.sender);
    }
}
```

### 3. **Verifiable Random Art Gallery**

Provably fair NFT collection where rarity is verifiable:

```solidity
contract VerifiableGallery {
    function submitArtwork(bytes calldata seal, bytes calldata journal) external {
        verifier.verifyImage(seal, journal);

        // Extract walks/steps (these determine rarity)
        (, uint64 walks, uint64 steps, ) =
            abi.decode(journal, (address, uint64, uint64, bytes));

        // Rarity based on deterministic parameters
        uint256 rarity = calculateRarity(walks, steps);

        // Store with verifiable rarity
        artworks[tokenId] = Artwork({
            owner: msg.sender,
            rarity: rarity,
            verified: true
        });
    }
}
```

## Cost Analysis

### Gas Costs (Ethereum Mainnet estimates)

| Operation | Gas | Cost @ 50 gwei | Cost @ $3000 ETH |
|-----------|-----|----------------|------------------|
| Deploy Verifier (one-time) | ~5M | 0.25 ETH | ~$750 |
| Verify Image | ~200K | 0.01 ETH | ~$30 |
| Batch Verify (5 images) | ~800K | 0.04 ETH | ~$120 |

### Bonsai Costs

- **Proof generation**: ~$0.10 - $1.00 per proof (varies by complexity)
- **Free tier**: 100 proofs/month for development

### Layer 2 Optimization

Deploy on L2 for 100x cheaper gas:

| Chain | Verify Cost | USD |
|-------|-------------|-----|
| Ethereum | 0.01 ETH | ~$30 |
| Arbitrum | 0.0001 ETH | ~$0.30 |
| Optimism | 0.0001 ETH | ~$0.30 |
| Base | 0.0001 ETH | ~$0.30 |

## Development Workflow

### Local Testing (No Bonsai needed)

```bash
# Generate STARK proof locally (fast)
cargo run -- --private-key 0x... --prove

# Verify locally (instant)
cargo run -- --verify image.proof
```

### Production (With Bonsai)

```bash
# Generate Groth16 proof via Bonsai (2-5 min)
cargo run --features bonsai -- --private-key 0x... --prove-groth16

# Deploy to testnet (Sepolia)
forge create RorschachVerifier --rpc-url $SEPOLIA_RPC

# Verify on-chain
cast send $CONTRACT "verifyImage(bytes,bytes)" \
  $(cat image.groth16) $(cat image.journal)
```

## Security Considerations

### ✅ What the proof guarantees:

1. **Private key ownership**: Prover knows the private key for the claimed address
2. **Deterministic generation**: Image was generated using the exact algorithm
3. **Parameter authenticity**: Walks/steps were derived from the private key
4. **Non-repudiation**: Once verified, proof cannot be revoked

### ⚠️ What the proof does NOT guarantee:

1. **Private key secrecy**: Don't include your real wallet's private key in proofs!
2. **Image uniqueness**: Same private key always generates the same image
3. **Timestamp**: Proof doesn't include when it was generated

### 🔒 Best Practices:

- **Use dedicated keys** for image generation (not your main wallet)
- **Verify contract addresses** before submitting proofs
- **Test on testnet first** (Sepolia, Goerli)
- **Monitor Bonsai credits** to avoid unexpected costs

## Troubleshooting

### "Bonsai API key not found"
```bash
export BONSAI_API_KEY="your_key"
export BONSAI_API_URL="https://api.bonsai.xyz"
```

### "Proof verification failed"
- Check that `ImageID` in contract matches the guest build
- Ensure journal format matches contract expectations
- Verify seal bytes are correct (not truncated)

### "Out of gas"
- Increase gas limit to 300K for verification
- Consider batching multiple verifications

## Advanced: Custom Verifier Logic

Extend the verifier for custom use cases:

```solidity
contract CustomRorschachVerifier is RorschachVerifier {
    // Require specific parameter ranges
    function verifyImageWithConstraints(
        bytes calldata seal,
        bytes calldata journal,
        uint64 minWalks,
        uint64 maxWalks
    ) external {
        verifier.verify(seal, imageId, sha256(journal));

        (, uint64 walks, , ) = abi.decode(
            journal,
            (address, uint64, uint64, bytes)
        );

        require(walks >= minWalks && walks <= maxWalks, "Invalid walks");

        // Custom logic here
    }
}
```

## Resources

- **risc0 Docs**: https://dev.risczero.com/
- **Bonsai Docs**: https://dev.bonsai.xyz/
- **Groth16 Verifier**: https://github.com/risc0/risc0/tree/main/bonsai/ethereum
- **Example Contracts**: https://github.com/risc0/risc0/tree/main/examples/ethereum

## Summary

**For Development:**
- Use local STARK proofs (fast, free)
- Test with `--prove` flag
- Verify in Rust instantly

**For Production:**
- Use Bonsai for Groth16 conversion
- Deploy verifier contracts
- Verify on-chain for ~$0.30 (L2) or ~$30 (L1)

The proof system enables **trustless verification** that an image was generated from a specific private key, without revealing the key itself!
