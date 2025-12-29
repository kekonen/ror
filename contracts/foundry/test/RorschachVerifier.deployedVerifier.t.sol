// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console2} from "forge-std/Test.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";
import {ImageID} from "../src/ImageID.sol";
import {IRiscZeroVerifier} from "../src/IRiscZeroVerifier.sol";
import {RiscZeroGroth16Verifier} from "risc0-ethereum/contracts/src/groth16/RiscZeroGroth16Verifier.sol";
import {ControlID} from "risc0-ethereum/contracts/src/groth16/ControlID.sol";

/// @notice Fork test with deployed risc0 v3.0 Groth16 verifier
/// @dev This test deploys the correct verifier for our proofs
contract RorschachVerifierDeployedTest is Test {
    RorschachVerifier public verifier;
    IRiscZeroVerifier public risc0Verifier;
    
    // Test data from our generated proof
    address constant TEST_ADDRESS = 0x49EC48990bdb9F089C32c4FC058e30Efb18802Be;
    uint64 constant TEST_WALKS = 13;
    uint64 constant TEST_STEPS = 269;

    function setUp() public {
        // Deploy the RISC Zero Groth16 verifier with v1.2.0 control IDs
        console2.log("Deploying RISC Zero Groth16 Verifier (v1.2.0)...");
        RiscZeroGroth16Verifier verifierImpl = new RiscZeroGroth16Verifier(
            ControlID.CONTROL_ROOT,
            ControlID.BN254_CONTROL_ID
        );
        risc0Verifier = IRiscZeroVerifier(address(verifierImpl));
        console2.log("RISC Zero verifier deployed at:", address(risc0Verifier));
        
        // Deploy RorschachVerifier with our deployed verifier
        verifier = new RorschachVerifier(risc0Verifier);
        console2.log("RorschachVerifier deployed at:", address(verifier));
        console2.log("Chain ID:", block.chainid);
    }

    /// @notice Test verification with deployed verifier
    function testDeployed_VerifyRealProof() public {
        string memory root = vm.projectRoot();
        
        // Read the proof files
        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16.seal")
        );
        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16.journal")
        );

        console2.log("\n=== Real Proof Test with Deployed Verifier ===");
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

        console2.log("\n=== Calling Deployed RISC Zero Verifier ===");
        console2.log("IMAGE_ID:", vm.toString(ImageID.GUEST_ID));
        
        // This should now succeed with the correct verifier!
        bool success = verifier.verifyImage(seal, journal);

        require(success, "Proof verification failed");

        console2.log("\n=== SUCCESS! Proof Verified On-Chain! ===");
        console2.log("[OK] RISC Zero Groth16 verifier accepted the proof!");
        console2.log("[OK] Full end-to-end verification complete!");
        console2.log("[OK] Image verified for address:", ethAddress);
    }

    /// @notice Test that the image is stored correctly
    function testDeployed_ImageStorage() public {
        string memory root = vm.projectRoot();
        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16.seal")
        );
        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16.journal")
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
