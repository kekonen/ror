// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console2} from "forge-std/Test.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";
import {ImageID} from "../src/ImageID.sol";
import {IRiscZeroVerifier, Receipt, VerificationFailed} from "../src/IRiscZeroVerifier.sol";

/// @notice Bypass verifier that accepts our Docker prover's selector
/// @dev This bypasses cryptographic verification to test contract logic
contract BypassVerifier is IRiscZeroVerifier {
    bytes4 public constant DOCKER_SELECTOR = 0x2f80f8d0;

    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest) external view {
        // Check selector matches
        require(bytes4(seal[:4]) == DOCKER_SELECTOR, "Selector mismatch");

        // For testing purposes, we skip cryptographic verification
        // In production, this would verify the Groth16 proof
    }

    function verifyIntegrity(Receipt calldata receipt) external view {
        require(bytes4(receipt.seal[:4]) == DOCKER_SELECTOR, "Selector mismatch");
        // Skip cryptographic verification for testing
    }
}

/// @notice Test with bypass verifier to validate contract integration
/// @dev This proves our Solidity contracts work correctly, pending resolution of the Docker VK mismatch
contract RorschachVerifierBypassTest is Test {
    RorschachVerifier public verifier;
    IRiscZeroVerifier public risc0Verifier;

    // Test data from our generated proof
    address constant TEST_ADDRESS = 0x70997970C51812dc3A010C7d01b50e0d17dc79C8;
    uint64 constant TEST_WALKS = 16;
    uint64 constant TEST_STEPS = 278;

    function setUp() public {
        // Deploy bypass verifier
        console2.log("Deploying Bypass Verifier (for integration testing)...");
        risc0Verifier = IRiscZeroVerifier(address(new BypassVerifier()));
        console2.log("Bypass verifier deployed at:", address(risc0Verifier));

        // Deploy RorschachVerifier
        verifier = new RorschachVerifier(risc0Verifier);
        console2.log("RorschachVerifier deployed at:", address(verifier));
    }

    /// @notice Test verification with bypass verifier
    /// @dev This validates contract logic without cryptographic verification
    function testBypass_VerifyRealProof() public {
        string memory root = vm.projectRoot();

        // Read the proof files (generated with risc0 v3.0.3)
        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16_v303.seal")
        );
        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16_v303.journal")
        );

        console2.log("\n=== Bypass Verifier Test (Integration Validation) ===");
        console2.log("Seal size:", seal.length);
        console2.log("Journal size:", journal.length);
        console2.log("Proof selector:", vm.toString(bytes4(seal[0:4])));

        // Decode and verify journal
        (address ethAddress, uint64 walks, uint64 steps, bytes memory binaryImage) =
            abi.decode(journal, (address, uint64, uint64, bytes));

        console2.log("\nDecoded Journal:");
        console2.log("  Address:", ethAddress);
        console2.log("  Walks:", walks);
        console2.log("  Steps:", steps);
        console2.log("  Binary image size:", binaryImage.length);

        // Verify the values match
        assertEq(ethAddress, TEST_ADDRESS, "Address mismatch");
        assertEq(walks, TEST_WALKS, "Walks mismatch");
        assertEq(steps, TEST_STEPS, "Steps mismatch");
        assertEq(binaryImage.length, 256, "Image size mismatch");

        console2.log("\n=== Calling Bypass Verifier ===");
        console2.log("IMAGE_ID:", vm.toString(ImageID.GUEST_ID));

        // This should succeed with the bypass verifier!
        bool success = verifier.verifyImage(seal, journal);

        require(success, "Proof verification failed");

        console2.log("\n=== SUCCESS! Contract Integration Validated! ===");
        console2.log("[OK] Solidity contract logic works correctly!");
        console2.log("[OK] Journal encoding/decoding verified!");
        console2.log("[OK] Image storage verified for address:", ethAddress);
        console2.log("\n[NOTE] This test bypasses cryptographic verification.");
        console2.log("[NOTE] Waiting for risc0 team to fix Docker image VK mismatch.");
    }

    /// @notice Test that the image is stored correctly
    function testBypass_ImageStorage() public {
        string memory root = vm.projectRoot();
        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16_v303.seal")
        );
        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16_v303.journal")
        );

        // Verify the proof
        verifier.verifyImage(seal, journal);

        // Check that the image was stored
        bytes32[] memory images = verifier.getAddressImages(TEST_ADDRESS);
        assertEq(images.length, 1, "Should have one image");

        // Verify the image is marked as verified
        bytes32 imageHash = images[0];
        assertTrue(verifier.verifiedImages(imageHash), "Image should be verified");

        console2.log("\n[OK] Image stored correctly");
        console2.log("Image hash:", vm.toString(imageHash));
    }
}
