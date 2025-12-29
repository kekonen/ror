// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test} from "forge-std/Test.sol";
import {console2} from "forge-std/console2.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";
import {IRiscZeroVerifier} from "../src/IRiscZeroVerifier.sol";

/// @notice Fork tests using real risc0 verifier on mainnet/testnet
/// @dev Run with: forge test --match-contract Fork --fork-url $ETH_RPC_URL
contract RorschachVerifierForkTest is Test {
    RorschachVerifier public verifier;

    // risc0 Groth16 verifier addresses
    // Source: https://dev.risczero.com/api/blockchain-integration/contracts/verifier
    address constant RISC0_VERIFIER_MAINNET = 0x8EaB2D97Dfce405A1692a21b3ff3A172d593D319;
    address constant RISC0_VERIFIER_SEPOLIA = 0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187;

    // Test data from our generated proof
    address constant TEST_ADDRESS = 0xfa3d428a0bc8B7A1AB5bB8B8ffbB6Ec78EBb31ED;
    uint64 constant TEST_WALKS = 6;
    uint64 constant TEST_STEPS = 131;

    function setUp() public {
        // Detect which network we're on and use appropriate verifier
        address verifierAddress;

        if (block.chainid == 1) {
            // Mainnet
            verifierAddress = RISC0_VERIFIER_MAINNET;
            console2.log("Using Mainnet risc0 verifier:", verifierAddress);
        } else if (block.chainid == 11155111) {
            // Sepolia
            verifierAddress = RISC0_VERIFIER_SEPOLIA;
            console2.log("Using Sepolia risc0 verifier:", verifierAddress);
        } else {
            // For local fork, try mainnet verifier
            verifierAddress = RISC0_VERIFIER_MAINNET;
            console2.log("Using Mainnet verifier on fork:", verifierAddress);
        }

        // Deploy our verifier contract with real risc0 verifier
        verifier = new RorschachVerifier(IRiscZeroVerifier(verifierAddress));

        console2.log("RorschachVerifier deployed at:", address(verifier));
        console2.log("Chain ID:", block.chainid);
    }

    /// @notice Test with real proof files on forked network
    /// @dev This will actually call the real risc0 Groth16 verifier!
    function testFork_VerifyRealProof() public {
        // Read the real proof files
        string memory root = vm.projectRoot();

        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16.seal")
        );

        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16.journal")
        );

        console2.log("=== Real Proof Test on Fork ===");
        console2.log("Seal size:", seal.length);
        console2.log("Journal size:", journal.length);

        // Try to decode journal - wrap in try/catch
        try this.decodeAndLogJournal(journal) {
            console2.log("Journal decoded successfully");
        } catch {
            console2.log("Journal decode failed (continuing with raw journal)");
        }

        // This will call the REAL risc0 verifier contract!
        // ImageID has been updated with the actual GUEST_ID from the build
        console2.log("\n=== Calling Real risc0 Verifier ===");

        bool success = verifier.verifyImage(seal, journal);

        require(success, "Proof verification failed");

        console2.log("\n=== SUCCESS! Proof Verified On-Chain! ===");
        console2.log("[OK] risc0 Groth16 verifier accepted the proof!");
        console2.log("[OK] Full end-to-end verification complete!");
    }

    /// @notice Helper function to decode and log journal (external for try/catch)
    function decodeAndLogJournal(bytes memory journal) external view {
        (address ethAddress, uint64 walks, uint64 steps, bytes memory binaryImage) =
            abi.decode(journal, (address, uint64, uint64, bytes));

        console2.log("Address:", ethAddress);
        console2.log("Walks:", walks);
        console2.log("Steps:", steps);
        console2.log("Binary image size:", binaryImage.length);
    }

    /// @notice Test ownership verification (view function)
    function testFork_VerifyOwnership() public {
        string memory root = vm.projectRoot();

        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16.seal")
        );

        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../../test_groth16.journal")
        );

        // Ownership check is a view function, will also fail with wrong ImageID
        vm.expectRevert();
        verifier.verifyOwnership(seal, journal, TEST_ADDRESS);

        console2.log("Ownership test completed (expected revert)");
    }

    /// @notice Test that verifier contract exists and is callable
    function testFork_VerifierExists() public view {
        IRiscZeroVerifier risc0Verifier = verifier.verifier();
        console2.log("risc0 Verifier address:", address(risc0Verifier));

        // Check the contract has code
        uint256 codeSize;
        address verifierAddr = address(risc0Verifier);
        assembly {
            codeSize := extcodesize(verifierAddr)
        }

        assertGt(codeSize, 0, "Verifier contract should have code");
        console2.log("Verifier code size:", codeSize, "bytes");
        console2.log("[OK] risc0 verifier contract exists on this network");
    }

    /// @notice Test basic contract functionality
    function testFork_BasicFunctionality() public {
        // Test that our contract is properly deployed
        assertEq(address(verifier.verifier()),
                 block.chainid == 11155111 ? RISC0_VERIFIER_SEPOLIA : RISC0_VERIFIER_MAINNET);

        // Test view functions work
        bytes32[] memory images = verifier.getAddressImages(TEST_ADDRESS);
        assertEq(images.length, 0, "Should have no verified images initially");

        assertFalse(verifier.isVerified(bytes32(0)), "Random hash should not be verified");

        console2.log("[OK] Basic contract functionality works on fork");
    }
}
