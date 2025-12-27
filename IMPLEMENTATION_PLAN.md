# On-Chain Verification Implementation Plan

## Overview

This plan adds local Groth16 proof generation (for x86 Linux with Docker) and comprehensive Foundry infrastructure with verifier contracts, testing suite, deployment scripts, and example use cases.

**Target Platform**: x86 Linux with Docker (your Linux machine)
**Development Platform**: macOS (STARK proofs + mock verifier testing)

## Current Status

✅ risc0 v3.0 STARK proofs working
✅ Binary optimization: 256 bytes (24× smaller than RGB)
✅ Guest commits ProofOutputs: address, walks, steps, binary_chunks
✅ Basic Solidity contract exists at `contracts/RorschachVerifier.sol`
❌ No Groth16 conversion (needed for on-chain verification)
❌ No Foundry testing/deployment infrastructure

## Technical Background

### Why Groth16?
- STARK proofs: 200KB-2MB (too large for on-chain)
- Groth16 proofs: ~500 bytes (EVM-compatible)
- Gas cost: ~250K gas (~$30 L1, ~$0.30 L2)

### Platform Constraint
**Critical**: Groth16 conversion requires x86 Linux + Docker
- Uses risc0's Docker-based prover: `risczero/risc0-groth16-prover`
- macOS ARM cannot run this (x86-only assembly in witness generator)
- This is why you'll implement on your Linux machine

---

## Phase 1: Rust Host Program - Groth16 Support

### Step 1.1: Update Dependencies

**File: `host/Cargo.toml`**

Add to `[dependencies]`:
```toml
risc0-groth16 = "3.0"
```

Add to `[features]`:
```toml
[features]
default = []
groth16 = ["risc0-groth16"]
```

### Step 1.2: Add CLI Flag

**File: `host/src/main.rs`**

After the existing `prove` field (around line 59), add:
```rust
/// Generate Groth16 proof for on-chain verification (requires x86 Linux + Docker)
#[arg(long)]
prove_groth16: bool,
```

### Step 1.3: Add Platform Detection

Add this function before `main()`:

```rust
fn check_groth16_platform() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(target_arch = "x86_64"))]
    {
        return Err("Groth16 proving requires x86_64 architecture. Current: ARM/other.
                   Please run on x86 Linux with Docker installed.".into());
    }

    #[cfg(target_arch = "x86_64")]
    {
        // Check if Docker is available
        let docker_check = std::process::Command::new("docker")
            .arg("--version")
            .output();

        match docker_check {
            Ok(output) if output.status.success() => Ok(()),
            _ => Err("Docker not found. Groth16 proving requires Docker.
                     Install: sudo apt-get install docker.io".into()),
        }
    }
}
```

### Step 1.4: Add Journal Encoding

Add these imports at the top:
```rust
use alloy::sol_types::{SolValue, sol};
```

Add this struct and function before `main()`:

```rust
// Define Solidity-compatible types for ABI encoding
sol! {
    struct JournalData {
        address ethAddress;
        uint64 walks;
        uint64 steps;
        bytes imageBytes;
    }
}

fn encode_journal_for_solidity(outputs: &ProofOutputs) -> Vec<u8> {
    // Flatten binary_chunks into single bytes array
    let mut image_bytes = Vec::with_capacity(256);
    for chunk in &outputs.binary_chunks {
        image_bytes.extend_from_slice(chunk);
    }

    // Convert address bytes to Alloy Address type
    let address = alloy::primitives::Address::from_slice(&outputs.address);

    // Create JournalData struct
    let journal_data = JournalData {
        ethAddress: address,
        walks: outputs.walks,
        steps: outputs.steps,
        imageBytes: image_bytes.into(),
    };

    // ABI-encode for Solidity compatibility
    journal_data.abi_encode()
}
```

### Step 1.5: Add Groth16 Proof Generation

Add this function after the existing `generate_proof()` function:

```rust
#[cfg(feature = "groth16")]
fn generate_groth16_proof(
    private_key: &[u8; 32],
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use risc0_zkvm::ProverOpts;
    use risc0_groth16::Groth16Prover;

    println!("Generating Groth16 proof... (this may take 5-10 minutes)");
    println!("Using Docker backend for Groth16 recursion...");

    // Build execution environment
    let env = ExecutorEnv::builder()
        .write(private_key)?
        .build()?;

    // Get prover with Groth16 backend
    let prover = Groth16Prover::new();
    let opts = ProverOpts::groth16();

    // Generate STARK proof first, then convert to Groth16
    println!("Step 1/2: Generating STARK proof...");
    let prove_info = prover.prove_with_opts(env, GUEST_ELF, &opts)?;

    println!("Step 2/2: Converting to Groth16 via Docker...");
    let receipt = prove_info.receipt;

    // Extract CompactReceipt
    let compact_receipt = receipt.inner.compact()
        .ok_or("Failed to get CompactReceipt")?;

    // Decode outputs
    let outputs: ProofOutputs = receipt.journal.decode()?;

    // Reconstruct binary image from chunks
    let mut binary_data = Vec::with_capacity(256);
    for chunk in &outputs.binary_chunks {
        binary_data.extend_from_slice(chunk);
    }
    let binary_image = BinaryImage32x64::from_bytes(&binary_data);

    println!("✓ Groth16 proof generated successfully!");
    println!("  Address: 0x{}", hex::encode(outputs.address));
    println!("  Parameters: walks={}, steps={}", outputs.walks, outputs.steps);
    println!("  Binary image size: {} bytes", binary_image.data.len());

    // Save seal (Groth16 proof) for on-chain verification
    let seal_path = output_path.with_extension("seal");
    fs::write(&seal_path, &compact_receipt.seal)?;
    println!("  Groth16 seal saved to: {} ({} bytes)", seal_path.display(), compact_receipt.seal.len());

    // Save journal (public outputs) for on-chain verification
    let journal_bytes = encode_journal_for_solidity(&outputs);
    let journal_path = output_path.with_extension("journal");
    fs::write(&journal_path, &journal_bytes)?;
    println!("  Journal saved to: {} ({} bytes)", journal_path.display(), journal_bytes.len());

    // Also save full receipt for reference
    let receipt_path = output_path.with_extension("groth16.proof");
    let receipt_bytes = bincode::serialize(&receipt)?;
    fs::write(&receipt_path, receipt_bytes)?;
    println!("  Full receipt saved to: {}", receipt_path.display());

    Ok(())
}

#[cfg(not(feature = "groth16"))]
fn generate_groth16_proof(
    _private_key: &[u8; 32],
    _output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Groth16 feature not enabled. Build with: cargo build --features groth16".into())
}
```

### Step 1.6: Update Main Function

Find the proof generation section (around line 265) and add this before the existing `if cli.prove` block:

```rust
// Groth16 proof generation mode (on-chain ready)
if cli.prove_groth16 {
    check_groth16_platform()?;
    generate_groth16_proof(&private_key, &cli.output)?;

    // Still generate the image for visualization
    let (default_walks, default_steps) = derive_parameters(&private_key);
    let walks = cli.walks.unwrap_or(default_walks);
    let steps = cli.steps.unwrap_or(default_steps);

    let foreground = cli.color.to_pixel();
    let background = cli.background.to_pixel();
    let half_image = generate_rorschach_half(&private_key, walks, steps, foreground, background);

    let mut full_image = mirror_half_to_full(&half_image, background);
    if !cli.no_stamp {
        add_corner_stamps(&mut full_image, &private_key,
            cli.color.to_rgb(), cli.background.to_rgb(), cli.stamp_offset);
    }
    let final_image = upscale(&full_image, 8);
    final_image.save(&cli.output)?;
    println!("  Image saved to: {}", cli.output.display());

    return Ok(());
}

// Existing STARK proof logic remains unchanged
if cli.prove {
    // ... existing code ...
}
```

---

## Phase 2: Foundry Project Setup

### Step 2.1: Create Directory Structure

**On your Linux machine**, create the Foundry project:

```bash
cd /path/to/ror
mkdir -p foundry/{src/examples,test,script}
```

### Step 2.2: Initialize Foundry

```bash
cd foundry
forge init --no-commit
```

### Step 2.3: Create foundry.toml

**File: `foundry/foundry.toml`**

```toml
[profile.default]
src = "src"
out = "out"
libs = ["lib"]
solc_version = "0.8.20"
optimizer = true
optimizer_runs = 200
via_ir = true

# Test configuration
gas_reports = ["*"]
gas_reports_ignore = ["test"]

# risc0 integration
fs_permissions = [{ access = "read", path = "../target" }]

[rpc_endpoints]
sepolia = "${SEPOLIA_RPC_URL}"
mainnet = "${MAINNET_RPC_URL}"
localhost = "http://127.0.0.1:8545"

[etherscan]
sepolia = { key = "${ETHERSCAN_API_KEY}" }
mainnet = { key = "${ETHERSCAN_API_KEY}" }
```

### Step 2.4: Install Dependencies

```bash
cd foundry
forge install foundry-rs/forge-std --no-commit
forge install risc0/risc0-ethereum@v2.1.0 --no-commit
```

### Step 2.5: Create Remappings

**File: `foundry/remappings.txt`**

```
forge-std/=lib/forge-std/src/
risc0/=lib/risc0-ethereum/contracts/src/
```

---

## Phase 3: Core Contracts

### Step 3.1: IRiscZeroVerifier Interface

**File: `foundry/src/IRiscZeroVerifier.sol`**

```solidity
// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

/// @notice Interface for risc0 verifier contracts
interface IRiscZeroVerifier {
    /// @notice Verify a proof with journal hash
    /// @param seal The encoded cryptographic proof
    /// @param imageId The identifier for the guest program
    /// @param journalDigest The SHA-256 digest of the journal
    function verify(
        bytes calldata seal,
        bytes32 imageId,
        bytes32 journalDigest
    ) external view;
}
```

### Step 3.2: ImageID Template

**File: `foundry/src/ImageID.sol`**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title ImageID
/// @notice Auto-generated from guest program build
/// @dev DO NOT EDIT MANUALLY - Will be updated by build process
library ImageID {
    /// @notice Image ID from methods/guest build
    /// @dev This is the digest of the guest ELF binary
    bytes32 public constant GUEST_ID = 0x0000000000000000000000000000000000000000000000000000000000000000;

    // NOTE: This will be replaced with actual GUEST_ID after building the guest program
    // Run: cargo build --release -p methods
    // Then update this value manually or via script
}
```

### Step 3.3: Main Verifier Contract

**File: `foundry/src/RorschachVerifier.sol`**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "./IRiscZeroVerifier.sol";
import {ImageID} from "./ImageID.sol";

/// @title RorschachVerifier
/// @notice Verifies Rorschach image proofs using risc0 Groth16
/// @dev Optimized for binary image format (256 bytes)
contract RorschachVerifier {
    IRiscZeroVerifier public immutable verifier;
    bytes32 public immutable imageId;

    struct ImageData {
        address ethAddress;
        uint64 walks;
        uint64 steps;
        bytes binaryImage;  // 256 bytes packed binary
    }

    event ImageVerified(
        bytes32 indexed imageHash,
        address indexed ethAddress,
        uint64 walks,
        uint64 steps,
        uint256 timestamp
    );

    mapping(bytes32 => bool) public verifiedImages;
    mapping(address => bytes32[]) public addressImages;

    error InvalidProof();
    error InvalidImageSize();
    error AlreadyVerified();

    constructor(IRiscZeroVerifier _verifier) {
        verifier = _verifier;
        imageId = ImageID.GUEST_ID;
    }

    /// @notice Verify a Groth16 proof of Rorschach generation
    /// @param seal The Groth16 proof seal from risc0
    /// @param journal The public outputs (ABI-encoded ImageData)
    function verifyImage(bytes calldata seal, bytes calldata journal)
        external
        returns (bool)
    {
        // Verify Groth16 proof
        verifier.verify(seal, imageId, sha256(journal));

        // Decode journal data
        (address ethAddress, uint64 walks, uint64 steps, bytes memory binaryImage) =
            abi.decode(journal, (address, uint64, uint64, bytes));

        // Validate binary image size (must be exactly 256 bytes)
        if (binaryImage.length != 256) revert InvalidImageSize();

        // Calculate image hash
        bytes32 imageHash = keccak256(binaryImage);

        // Check not already verified
        if (verifiedImages[imageHash]) revert AlreadyVerified();

        // Store verification
        verifiedImages[imageHash] = true;
        addressImages[ethAddress].push(imageHash);

        emit ImageVerified(imageHash, ethAddress, walks, steps, block.timestamp);

        return true;
    }

    /// @notice Verify ownership without storing (view function)
    /// @param seal The Groth16 proof seal
    /// @param journal The public outputs
    /// @param claimedAddress The address to verify
    function verifyOwnership(
        bytes calldata seal,
        bytes calldata journal,
        address claimedAddress
    ) external view returns (bool) {
        verifier.verify(seal, imageId, sha256(journal));

        (address ethAddress, , , ) = abi.decode(
            journal,
            (address, uint64, uint64, bytes)
        );

        return ethAddress == claimedAddress;
    }

    /// @notice Get all verified images for an address
    function getAddressImages(address addr)
        external
        view
        returns (bytes32[] memory)
    {
        return addressImages[addr];
    }

    /// @notice Check if an image has been verified
    function isVerified(bytes32 imageHash) external view returns (bool) {
        return verifiedImages[imageHash];
    }
}
```

---

## Phase 4: Example Contracts

### Step 4.1: RorschachNFT

**File: `foundry/src/examples/RorschachNFT.sol`**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ERC721} from "forge-std/lib/openzeppelin-contracts/contracts/token/ERC721/ERC721.sol";
import {RorschachVerifier} from "../RorschachVerifier.sol";

/// @title RorschachNFT
/// @notice Mint NFTs by proving private key ownership
/// @dev Each unique Rorschach pattern becomes an NFT
contract RorschachNFT is ERC721 {
    RorschachVerifier public immutable verifier;
    uint256 public nextTokenId;

    mapping(bytes32 => uint256) public imageHashToTokenId;
    mapping(uint256 => bytes) public tokenIdToBinaryImage;
    mapping(uint256 => uint64) public tokenIdToWalks;
    mapping(uint256 => uint64) public tokenIdToSteps;

    event RorschachMinted(
        uint256 indexed tokenId,
        address indexed owner,
        bytes32 imageHash,
        uint64 walks,
        uint64 steps
    );

    error AlreadyMinted();
    error ProofVerificationFailed();

    constructor(address _verifier) ERC721("Rorschach", "RSCH") {
        verifier = RorschachVerifier(_verifier);
    }

    /// @notice Mint NFT by proving Rorschach generation
    /// @param seal Groth16 proof seal
    /// @param journal Public outputs
    function mintWithProof(bytes calldata seal, bytes calldata journal)
        external
        returns (uint256 tokenId)
    {
        // Verify the proof
        try verifier.verifyImage(seal, journal) returns (bool) {
            // Proof is valid, continue
        } catch {
            revert ProofVerificationFailed();
        }

        // Decode journal
        (address ethAddress, uint64 walks, uint64 steps, bytes memory binaryImage) =
            abi.decode(journal, (address, uint64, uint64, bytes));

        require(ethAddress == msg.sender, "Proof must be for your address");

        bytes32 imageHash = keccak256(binaryImage);

        // Check not already minted
        if (imageHashToTokenId[imageHash] != 0) revert AlreadyMinted();

        // Mint NFT
        tokenId = nextTokenId++;
        _safeMint(msg.sender, tokenId);

        // Store metadata
        imageHashToTokenId[imageHash] = tokenId;
        tokenIdToBinaryImage[tokenId] = binaryImage;
        tokenIdToWalks[tokenId] = walks;
        tokenIdToSteps[tokenId] = steps;

        emit RorschachMinted(tokenId, msg.sender, imageHash, walks, steps);
    }

    /// @notice Get NFT metadata
    function getMetadata(uint256 tokenId)
        external
        view
        returns (
            bytes memory binaryImage,
            uint64 walks,
            uint64 steps,
            address owner
        )
    {
        require(_ownerOf(tokenId) != address(0), "Token doesn't exist");

        return (
            tokenIdToBinaryImage[tokenId],
            tokenIdToWalks[tokenId],
            tokenIdToSteps[tokenId],
            ownerOf(tokenId)
        );
    }

    /// @notice Calculate rarity score (higher walks/steps = rarer)
    function getRarityScore(uint256 tokenId) external view returns (uint256) {
        require(_ownerOf(tokenId) != address(0), "Token doesn't exist");

        uint64 walks = tokenIdToWalks[tokenId];
        uint64 steps = tokenIdToSteps[tokenId];

        // Simple rarity: walks * steps
        // Higher values = more complex pattern = rarer
        return uint256(walks) * uint256(steps);
    }
}
```

### Step 4.2: VerifiableGallery

**File: `foundry/src/examples/VerifiableGallery.sol`**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {RorschachVerifier} from "../RorschachVerifier.sol";

/// @title VerifiableGallery
/// @notice Provably fair art gallery with verifiable rarity
contract VerifiableGallery {
    RorschachVerifier public immutable verifier;

    struct Artwork {
        address creator;
        bytes32 imageHash;
        uint64 walks;
        uint64 steps;
        uint256 rarityScore;
        uint256 submittedAt;
        bool forSale;
        uint256 price;
    }

    uint256 public nextArtworkId;
    mapping(uint256 => Artwork) public artworks;
    mapping(bytes32 => uint256) public imageHashToArtworkId;

    event ArtworkSubmitted(
        uint256 indexed artworkId,
        address indexed creator,
        bytes32 imageHash,
        uint256 rarityScore
    );

    event ArtworkListed(uint256 indexed artworkId, uint256 price);
    event ArtworkSold(uint256 indexed artworkId, address buyer, uint256 price);

    constructor(address _verifier) {
        verifier = RorschachVerifier(_verifier);
    }

    /// @notice Submit artwork with proof
    function submitArtwork(bytes calldata seal, bytes calldata journal)
        external
        returns (uint256 artworkId)
    {
        // Verify proof
        verifier.verifyImage(seal, journal);

        // Decode data
        (address ethAddress, uint64 walks, uint64 steps, bytes memory binaryImage) =
            abi.decode(journal, (address, uint64, uint64, bytes));

        require(ethAddress == msg.sender, "Must use your own private key");

        bytes32 imageHash = keccak256(binaryImage);
        require(imageHashToArtworkId[imageHash] == 0, "Already submitted");

        // Calculate provable rarity
        uint256 rarityScore = calculateRarity(walks, steps);

        // Create artwork
        artworkId = nextArtworkId++;
        artworks[artworkId] = Artwork({
            creator: msg.sender,
            imageHash: imageHash,
            walks: walks,
            steps: steps,
            rarityScore: rarityScore,
            submittedAt: block.timestamp,
            forSale: false,
            price: 0
        });

        imageHashToArtworkId[imageHash] = artworkId;

        emit ArtworkSubmitted(artworkId, msg.sender, imageHash, rarityScore);
    }

    /// @notice List artwork for sale
    function listArtwork(uint256 artworkId, uint256 price) external {
        Artwork storage artwork = artworks[artworkId];
        require(artwork.creator == msg.sender, "Not creator");
        require(!artwork.forSale, "Already listed");

        artwork.forSale = true;
        artwork.price = price;

        emit ArtworkListed(artworkId, price);
    }

    /// @notice Purchase artwork
    function purchaseArtwork(uint256 artworkId) external payable {
        Artwork storage artwork = artworks[artworkId];
        require(artwork.forSale, "Not for sale");
        require(msg.value == artwork.price, "Incorrect price");

        address creator = artwork.creator;
        artwork.creator = msg.sender;
        artwork.forSale = false;
        artwork.price = 0;

        // Transfer payment to creator
        (bool success, ) = creator.call{value: msg.value}("");
        require(success, "Transfer failed");

        emit ArtworkSold(artworkId, msg.sender, msg.value);
    }

    /// @notice Calculate provable rarity score
    /// @dev Higher walks * steps = more complex = rarer
    function calculateRarity(uint64 walks, uint64 steps)
        public
        pure
        returns (uint256)
    {
        // Base score: walks * steps (max ~20 * 300 = 6000)
        uint256 baseScore = uint256(walks) * uint256(steps);

        // Bonus for high walks (encourages complexity)
        uint256 walksBonus = walks > 15 ? (walks - 15) * 100 : 0;

        // Bonus for high steps
        uint256 stepsBonus = steps > 250 ? (steps - 250) * 10 : 0;

        return baseScore + walksBonus + stepsBonus;
    }
}
```

---

## Phase 5: Testing Infrastructure

### Step 5.1: Mock Verifier for Fast Testing

**File: `foundry/test/RorschachVerifier.t.sol`**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test} from "forge-std/Test.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";
import {IRiscZeroVerifier} from "../src/IRiscZeroVerifier.sol";

/// @notice Mock verifier for testing
contract MockRiscZeroVerifier is IRiscZeroVerifier {
    bool public shouldRevert;

    function setShouldRevert(bool _shouldRevert) external {
        shouldRevert = _shouldRevert;
    }

    function verify(
        bytes calldata,
        bytes32,
        bytes32
    ) external view override {
        if (shouldRevert) revert("Mock verification failed");
    }
}

contract RorschachVerifierTest is Test {
    RorschachVerifier public verifier;
    MockRiscZeroVerifier public mockVerifier;

    address constant TEST_ADDRESS = 0x1111111111111111111111111111111111111111;
    uint64 constant TEST_WALKS = 10;
    uint64 constant TEST_STEPS = 150;

    function setUp() public {
        mockVerifier = new MockRiscZeroVerifier();
        verifier = new RorschachVerifier(IRiscZeroVerifier(address(mockVerifier)));
    }

    function testVerifyImage() public {
        // Create test journal data
        bytes memory binaryImage = new bytes(256);
        for (uint i = 0; i < 256; i++) {
            binaryImage[i] = bytes1(uint8(i));
        }

        bytes memory journal = abi.encode(
            TEST_ADDRESS,
            TEST_WALKS,
            TEST_STEPS,
            binaryImage
        );

        bytes memory seal = hex"1234"; // Mock seal

        // Verify image
        bool success = verifier.verifyImage(seal, journal);
        assertTrue(success);

        // Check storage
        bytes32 imageHash = keccak256(binaryImage);
        assertTrue(verifier.isVerified(imageHash));

        // Check address mapping
        bytes32[] memory images = verifier.getAddressImages(TEST_ADDRESS);
        assertEq(images.length, 1);
        assertEq(images[0], imageHash);
    }

    function testVerifyImageInvalidSize() public {
        bytes memory binaryImage = new bytes(100); // Wrong size
        bytes memory journal = abi.encode(TEST_ADDRESS, TEST_WALKS, TEST_STEPS, binaryImage);
        bytes memory seal = hex"1234";

        vm.expectRevert(RorschachVerifier.InvalidImageSize.selector);
        verifier.verifyImage(seal, journal);
    }

    function testVerifyImageAlreadyVerified() public {
        bytes memory binaryImage = new bytes(256);
        bytes memory journal = abi.encode(TEST_ADDRESS, TEST_WALKS, TEST_STEPS, binaryImage);
        bytes memory seal = hex"1234";

        verifier.verifyImage(seal, journal);

        vm.expectRevert(RorschachVerifier.AlreadyVerified.selector);
        verifier.verifyImage(seal, journal);
    }

    function testVerifyOwnership() public {
        bytes memory binaryImage = new bytes(256);
        bytes memory journal = abi.encode(TEST_ADDRESS, TEST_WALKS, TEST_STEPS, binaryImage);
        bytes memory seal = hex"1234";

        bool isOwner = verifier.verifyOwnership(seal, journal, TEST_ADDRESS);
        assertTrue(isOwner);

        bool notOwner = verifier.verifyOwnership(seal, journal, address(0x2222));
        assertFalse(notOwner);
    }

    function testVerifyImageRevertsOnBadProof() public {
        mockVerifier.setShouldRevert(true);

        bytes memory binaryImage = new bytes(256);
        bytes memory journal = abi.encode(TEST_ADDRESS, TEST_WALKS, TEST_STEPS, binaryImage);
        bytes memory seal = hex"1234";

        vm.expectRevert("Mock verification failed");
        verifier.verifyImage(seal, journal);
    }

    function testGasUsage() public {
        bytes memory binaryImage = new bytes(256);
        bytes memory journal = abi.encode(TEST_ADDRESS, TEST_WALKS, TEST_STEPS, binaryImage);
        bytes memory seal = hex"1234";

        uint256 gasBefore = gasleft();
        verifier.verifyImage(seal, journal);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("Gas used for verification", gasUsed);
    }
}
```

### Step 5.2: Integration Tests with Real Proofs

**File: `foundry/test/Integration.t.sol`**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test} from "forge-std/Test.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";
import {IRiscZeroVerifier} from "../src/IRiscZeroVerifier.sol";

/// @notice Integration tests with real Groth16 proofs
/// @dev Run on Linux after generating real proofs
contract IntegrationTest is Test {
    RorschachVerifier public verifier;

    function setUp() public {
        // Get risc0 verifier address from environment
        address verifierAddress = vm.envOr("RISC0_VERIFIER_ADDRESS", address(0));

        if (verifierAddress == address(0)) {
            // Skip if no real verifier available
            vm.skip(true);
            return;
        }

        verifier = new RorschachVerifier(IRiscZeroVerifier(verifierAddress));
    }

    function testRealProofVerification() public {
        // Load real proof files generated by host program
        string memory root = vm.projectRoot();

        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../test.seal")
        );

        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../test.journal")
        );

        // Verify real proof
        bool success = verifier.verifyImage(seal, journal);
        assertTrue(success);

        emit log_string("Successfully verified real Groth16 proof!");
    }
}
```

---

## Phase 6: Deployment Scripts

### Step 6.1: Deployment Script

**File: `foundry/script/Deploy.s.sol`**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script} from "forge-std/Script.sol";
import {console2} from "forge-std/console2.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";
import {RorschachNFT} from "../src/examples/RorschachNFT.sol";
import {VerifiableGallery} from "../src/examples/VerifiableGallery.sol";
import {IRiscZeroVerifier} from "../src/IRiscZeroVerifier.sol";

contract Deploy is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address risc0Verifier = vm.envAddress("RISC0_VERIFIER_ADDRESS");

        vm.startBroadcast(deployerPrivateKey);

        // Deploy main verifier
        RorschachVerifier verifier = new RorschachVerifier(
            IRiscZeroVerifier(risc0Verifier)
        );
        console2.log("RorschachVerifier deployed at:", address(verifier));

        // Deploy example NFT
        RorschachNFT nft = new RorschachNFT(address(verifier));
        console2.log("RorschachNFT deployed at:", address(nft));

        // Deploy gallery
        VerifiableGallery gallery = new VerifiableGallery(address(verifier));
        console2.log("VerifiableGallery deployed at:", address(gallery));

        vm.stopBroadcast();

        // Save deployment addresses
        string memory addresses = string.concat(
            "RORSCHACH_VERIFIER=", vm.toString(address(verifier)), "\n",
            "RORSCHACH_NFT=", vm.toString(address(nft)), "\n",
            "VERIFIABLE_GALLERY=", vm.toString(address(gallery)), "\n"
        );

        vm.writeFile(".env.deployed", addresses);
        console2.log("\nDeployment addresses saved to .env.deployed");
    }
}
```

### Step 6.2: Proof Submission Script

**File: `foundry/script/SubmitProof.s.sol`**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script} from "forge-std/Script.sol";
import {console2} from "forge-std/console2.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";

contract SubmitProof is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address verifierAddress = vm.envAddress("RORSCHACH_VERIFIER");

        // Load proof files
        string memory root = vm.projectRoot();
        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../", vm.envString("SEAL_FILE"))
        );
        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../", vm.envString("JOURNAL_FILE"))
        );

        console2.log("Submitting proof to verifier at:", verifierAddress);
        console2.log("Seal size:", seal.length, "bytes");
        console2.log("Journal size:", journal.length, "bytes");

        vm.startBroadcast(deployerPrivateKey);

        RorschachVerifier verifier = RorschachVerifier(verifierAddress);
        bool success = verifier.verifyImage(seal, journal);

        vm.stopBroadcast();

        require(success, "Proof verification failed");
        console2.log("\n✓ Proof verified successfully on-chain!");
    }
}
```

---

## Phase 7: Build Automation

### Step 7.1: Makefile

**File: `foundry/Makefile`**

```makefile
.PHONY: install build test deploy-sepolia submit-proof clean

# Install dependencies
install:
	forge install foundry-rs/forge-std --no-commit
	forge install risc0/risc0-ethereum --no-commit

# Build contracts
build:
	forge build

# Run tests with mock verifier (fast)
test:
	forge test -vvv

# Run integration tests with real proofs (requires proofs generated)
test-integration:
	forge test --match-contract Integration -vvv

# Deploy to Sepolia testnet
deploy-sepolia:
	@echo "Deploying to Sepolia..."
	@echo "Make sure RISC0_VERIFIER_ADDRESS is set!"
	forge script script/Deploy.s.sol --rpc-url sepolia --broadcast --verify

# Submit proof to deployed contract
submit-proof:
	@echo "Submitting proof..."
	@echo "Make sure SEAL_FILE and JOURNAL_FILE are set!"
	forge script script/SubmitProof.s.sol --rpc-url $(RPC_URL) --broadcast

# Generate local Groth16 proof for testing (on Linux only)
generate-proof:
	cd .. && cargo run --release --features groth16 --bin ror -- \
		--private-key $(PRIVATE_KEY) \
		--prove-groth16 \
		--output test.png

# Clean build artifacts
clean:
	forge clean
	rm -rf cache out

# Help
help:
	@echo "Available commands:"
	@echo "  make install          - Install Foundry dependencies"
	@echo "  make build            - Build contracts"
	@echo "  make test             - Run unit tests (mock verifier)"
	@echo "  make test-integration - Run integration tests (real proofs)"
	@echo "  make deploy-sepolia   - Deploy to Sepolia testnet"
	@echo "  make submit-proof     - Submit proof to deployed contract"
	@echo "  make generate-proof   - Generate Groth16 proof (Linux only)"
	@echo "  make clean            - Clean build artifacts"
```

### Step 7.2: README

**File: `foundry/README.md`**

```markdown
# Rorschach Foundry Infrastructure

Comprehensive testing, deployment, and verification infrastructure for Rorschach zkVM proofs.

## Quick Start

```bash
# Install dependencies
make install

# Build contracts
make build

# Run tests (with mock verifier)
make test
```

## Project Structure

- `src/` - Smart contracts
  - `RorschachVerifier.sol` - Main verifier contract
  - `ImageID.sol` - Guest program image ID
  - `examples/` - Example use cases (NFT, Gallery)
- `test/` - Test contracts (unit + integration)
- `script/` - Deployment and utility scripts

## Testing

### Unit Tests (Fast - Mock Verifier)
```bash
make test
```

### Integration Tests (Real Proofs - Linux only)
```bash
# 1. Generate proof
make generate-proof PRIVATE_KEY=0x...

# 2. Run integration tests
make test-integration
```

## Deployment

### Sepolia Testnet

```bash
export PRIVATE_KEY=0x...
export RISC0_VERIFIER_ADDRESS=0x...  # risc0 Groth16 verifier on Sepolia
make deploy-sepolia
```

### Submit Proof

```bash
export RORSCHACH_VERIFIER=0x...  # Your deployed verifier
export SEAL_FILE=test.seal
export JOURNAL_FILE=test.journal
export RPC_URL=https://sepolia.infura.io/v3/...
make submit-proof
```

## Gas Costs (Estimates)

| Operation | Gas | Cost @ 50 gwei | USD @ $3000 ETH |
|-----------|-----|----------------|-----------------|
| Deploy Verifier | ~800K | 0.04 ETH | ~$120 |
| Verify Image | ~250K | 0.0125 ETH | ~$38 |
| Mint NFT (with proof) | ~350K | 0.0175 ETH | ~$53 |

L2 costs are ~100× cheaper (Arbitrum, Optimism, Base).

## Example Contracts

### RorschachNFT
Mint NFTs by proving private key ownership. Each unique pattern becomes an NFT.

### VerifiableGallery
Provably fair art gallery with verifiable rarity scores.

## Troubleshooting

### "ImageID mismatch"
Guest program changed. Rebuild guest and update `src/ImageID.sol` with new GUEST_ID.

### "Proof verification failed"
- Ensure seal and journal match
- Check ImageID matches guest build
- Verify using correct verifier contract

### "Out of gas"
Increase gas limit: `--gas-limit 500000`
```

---

## Phase 8: Usage Guide

Create a comprehensive user guide for running on Linux.

**File: `GROTH16_USAGE_GUIDE.md`** (in project root)

```markdown
# Groth16 Proof Generation & On-Chain Verification

## Prerequisites (Linux Only)

### Check Your System

```bash
# Must show: Linux x86_64
uname -sm

# Must show Docker version
docker --version
```

If Docker not installed:
```bash
sudo apt-get update
sudo apt-get install docker.io
sudo usermod -aG docker $USER
# Log out and back in for group changes
```

## Step-by-Step Guide

### 1. Build with Groth16 Support

```bash
cd /path/to/ror
cargo build --release --features groth16
```

### 2. Generate Groth16 Proof

```bash
./target/release/ror \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove-groth16 \
  --output my_proof.png
```

**Expected duration**: 5-10 minutes
**Expected output**:
- `my_proof.png` - Visual image (512×512)
- `my_proof.seal` - Groth16 proof (~500 bytes)
- `my_proof.journal` - Public outputs (ABI-encoded)
- `my_proof.groth16.proof` - Full receipt

### 3. Test Locally with Foundry

```bash
cd foundry

# Run unit tests (fast, mock verifier)
make test

# Run integration test with your real proof
cp ../my_proof.seal ../test.seal
cp ../my_proof.journal ../test.journal
make test-integration
```

### 4. Deploy to Testnet

Get Sepolia testnet ETH from https://sepoliafaucet.com

```bash
cd foundry

# Set environment variables
export PRIVATE_KEY=0x...  # Your deployer key (with Sepolia ETH)
export RISC0_VERIFIER_ADDRESS=0x...  # risc0's Sepolia verifier

# Deploy contracts
make deploy-sepolia

# Output will show deployed addresses
# RORSCHACH_VERIFIER=0x...
# RORSCHACH_NFT=0x...
# VERIFIABLE_GALLERY=0x...
```

### 5. Submit Proof On-Chain

```bash
export RORSCHACH_VERIFIER=0x...  # From deployment
export SEAL_FILE=my_proof.seal
export JOURNAL_FILE=my_proof.journal
export RPC_URL=https://sepolia.infura.io/v3/YOUR_KEY

make submit-proof
```

**Success output:**
```
✓ Proof verified successfully on-chain!
```

## Cost Breakdown

### Generation Costs
- Local Groth16: **Free** (just electricity for 5-10 min)
- No cloud API fees (unlike Bonsai)

### On-Chain Costs (Sepolia Testnet)
- Deployment: ~0.04 ETH (~$120 mainnet)
- Proof verification: ~0.0125 ETH (~$38 mainnet)
- **Testnet**: Free with faucet ETH

### Production Costs
| Chain | Verify Cost | USD Equivalent |
|-------|-------------|----------------|
| Ethereum L1 | ~0.0125 ETH | ~$38 |
| Arbitrum L2 | ~0.0001 ETH | ~$0.30 |
| Base L2 | ~0.0001 ETH | ~$0.30 |

## Troubleshooting

### "Groth16 requires x86_64"
You're on wrong architecture. Must use x86 Linux.

```bash
uname -sm
# Must show: Linux x86_64
```

### "Docker not found"
Install Docker:
```bash
sudo apt-get install docker.io
sudo usermod -aG docker $USER
# Re-login
```

### "Permission denied" (Docker)
Add yourself to docker group:
```bash
sudo usermod -aG docker $USER
newgrp docker
```

### Proof generation stalls at "Step 2/2"
Docker prover issue. Check logs:
```bash
docker logs $(docker ps -a -q --filter ancestor=risczero/risc0-groth16-prover | head -1)
```

### "ImageID mismatch" on-chain
Guest program changed. Update ImageID.sol:

```bash
# 1. Rebuild guest
cargo build --release -p methods

# 2. Get new GUEST_ID
# Extract from target/riscv-guest/release/methods.rs

# 3. Update foundry/src/ImageID.sol with new ID

# 4. Redeploy contracts
cd foundry && make deploy-sepolia
```

## Development Workflow

### Quick Iteration (macOS/Linux - STARK only)
```bash
RISC0_DEV_MODE=1 cargo run --bin ror -- --prove --output test.png
```

### Production Proofs (Linux only)
```bash
cargo run --release --features groth16 --bin ror -- \
  --prove-groth16 \
  --output production.png
```

### Testing Flow
1. **Unit tests** (fast): `cd foundry && make test`
2. **Generate proof**: `make generate-proof PRIVATE_KEY=0x...`
3. **Integration tests**: `make test-integration`
4. **Deploy to testnet**: `make deploy-sepolia`
5. **Submit proof**: `make submit-proof`

## Next Steps

- **Mint NFT**: Use RorschachNFT contract to mint from proof
- **Create gallery**: Submit to VerifiableGallery with provable rarity
- **Build apps**: Use contracts as foundation for your dApp

## Architecture Overview

```
Private Key
    ↓
[Guest Program] → STARK Proof (2-5 min)
    ↓
[Docker Groth16] → CompactReceipt (3-8 min)
    ↓
seal (500 bytes) + journal (ABI-encoded)
    ↓
[EVM Verifier] → On-chain verification (250K gas)
```

## Support

- risc0 Docs: https://dev.risczero.com
- Foundry Book: https://book.getfoundry.sh
- Project Issues: https://github.com/your-repo/issues
```

---

## Implementation Checklist

### Phase 1: Rust Changes ✅
- [ ] Update `host/Cargo.toml` with risc0-groth16
- [ ] Add `--prove-groth16` CLI flag
- [ ] Add platform detection function
- [ ] Add journal encoding function
- [ ] Add Groth16 proof generation function
- [ ] Update main function to handle new flag

### Phase 2: Foundry Setup ✅
- [ ] Create foundry directory structure
- [ ] Initialize Foundry project
- [ ] Create foundry.toml
- [ ] Install forge dependencies
- [ ] Create remappings.txt

### Phase 3: Core Contracts ✅
- [ ] Create IRiscZeroVerifier.sol
- [ ] Create ImageID.sol template
- [ ] Create RorschachVerifier.sol

### Phase 4: Examples ✅
- [ ] Create RorschachNFT.sol
- [ ] Create VerifiableGallery.sol

### Phase 5: Testing ✅
- [ ] Create RorschachVerifier.t.sol (unit tests)
- [ ] Create Integration.t.sol (real proofs)

### Phase 6: Deployment ✅
- [ ] Create Deploy.s.sol
- [ ] Create SubmitProof.s.sol

### Phase 7: Automation ✅
- [ ] Create Makefile
- [ ] Create foundry/README.md

### Phase 8: Documentation ✅
- [ ] Create GROTH16_USAGE_GUIDE.md

## Testing the Implementation

### On Linux Machine

```bash
# 1. Build with Groth16
cargo build --release --features groth16

# 2. Generate a proof
./target/release/ror \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove-groth16 \
  --output test.png

# Expected output:
# ✓ Groth16 proof generated successfully!
#   Address: 0x19e7e376e7c213b7e7e7e46cc70a5dd086daff2a
#   Parameters: walks=5, steps=211
#   Groth16 seal saved to: test.seal (480 bytes)
#   Journal saved to: test.journal (352 bytes)

# 3. Test Foundry contracts
cd foundry
make install
make build
make test

# 4. Copy proof for integration test
cp ../test.seal ../test.seal
cp ../test.journal ../test.journal

# Expected: All tests pass ✓
```

## Key Technical Points

### Journal Encoding
- Rust uses Alloy's `sol!` macro for ABI encoding
- Solidity uses `abi.decode(journal, (address, uint64, uint64, bytes))`
- Binary chunks (8×32 bytes) flattened to single `bytes` array

### Platform Detection
- `#[cfg(not(target_arch = "x86_64"))]` catches non-x86
- Docker check ensures `docker --version` succeeds
- Clear error messages guide users

### Testing Strategy
- **Mock verifier**: Fast unit tests, no proof needed
- **Real verifier**: Integration tests with actual Groth16 proofs
- **Testnet**: End-to-end validation

## Success Criteria

✅ Rust program builds with `--features groth16`
✅ Groth16 proof generation completes in 5-10 minutes
✅ seal + journal files created correctly
✅ Foundry tests pass (unit + integration)
✅ Can deploy to Sepolia testnet
✅ Can verify proof on-chain successfully

## Timeline Estimate

- **Phase 1** (Rust): 2-3 hours
- **Phase 2-3** (Foundry setup + contracts): 2-3 hours
- **Phase 4** (Examples): 1-2 hours
- **Phase 5** (Testing): 1-2 hours
- **Phase 6-7** (Deployment + automation): 1 hour
- **Phase 8** (Documentation): 1 hour

**Total**: ~8-12 hours of implementation time

## Notes

- Proof generation takes 5-10 minutes per proof on Linux
- First Docker pull may take extra time (~2GB image)
- Testnet deployment requires Sepolia ETH (free from faucet)
- ImageID must match between guest build and Solidity contract
- All testing can be done locally before deploying to testnet
