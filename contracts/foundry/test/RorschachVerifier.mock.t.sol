// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console2} from "forge-std/Test.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";
import {ImageID} from "../src/ImageID.sol";
import {IRiscZeroVerifier} from "../src/IRiscZeroVerifier.sol";
import {RiscZeroMockVerifier} from "risc0-ethereum/contracts/src/test/RiscZeroMockVerifier.sol";

/// @notice Test with mock verifier configured to match our proof's selector
/// @dev This validates our integration works, pending resolution of the Groth16 VK mismatch
contract RorschachVerifierMockTest is Test {
    RorschachVerifier public verifier;
    IRiscZeroVerifier public risc0Verifier;

    // Test data from our generated proof (test_groth16_clean)
    address constant TEST_ADDRESS = 0x70997970C51812dc3A010C7d01b50e0d17dc79C8;
    uint64 constant TEST_WALKS = 16;
    uint64 constant TEST_STEPS = 278;

    // Selector from our Docker-generated proof
    bytes4 constant PROOF_SELECTOR = 0x1f1d6202;

    function setUp() public {
        // Deploy mock verifier with selector matching our proof
        console2.log("Deploying RiscZeroMockVerifier with selector:", vm.toString(PROOF_SELECTOR));
        risc0Verifier = IRiscZeroVerifier(address(new RiscZeroMockVerifier(PROOF_SELECTOR)));
        console2.log("Mock verifier deployed at:", address(risc0Verifier));

        // Deploy RorschachVerifier with mock verifier
        verifier = new RorschachVerifier(risc0Verifier);
        console2.log("RorschachVerifier deployed at:", address(verifier));
    }

    /// @notice Test verification with mock verifier
    /// @dev This proves our contract integration works correctly
    function testMock_VerifyRealProof() public {
        string memory root = vm.projectRoot();

        // Read the proof files
        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16_clean.seal")
        );
        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16_clean.journal")
        );

        console2.log("\n=== Mock Verifier Test ===");
        console2.log("Seal size:", seal.length);
        console2.log("Journal size:", journal.length);

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

        console2.log("\n=== Calling Mock Verifier ===");
        console2.log("IMAGE_ID:", vm.toString(ImageID.GUEST_ID));

        // This should succeed with the mock verifier!
        bool success = verifier.verifyImage(seal, journal);

        require(success, "Proof verification failed");

        console2.log("\n=== SUCCESS! Mock Verification Complete! ===");
        console2.log("[OK] Contract integration verified!");
        console2.log("[OK] Journal decoding works correctly!");
        console2.log("[OK] Image verified for address:", ethAddress);
        console2.log("\nNote: This uses a mock verifier. For production, resolve the");
        console2.log("Groth16 verification key mismatch between Docker prover and risc0-ethereum.");
    }

    /// @notice Test that the image is stored correctly
    function testMock_ImageStorage() public {
        string memory root = vm.projectRoot();
        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16_clean.seal")
        );
        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16_clean.journal")
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
