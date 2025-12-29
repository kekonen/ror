// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script} from "forge-std/Script.sol";
import {console2} from "forge-std/console2.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";

/// @notice Script to submit a Groth16 proof to deployed verifier
/// @dev Run with: forge script script/SubmitProof.s.sol --rpc-url <network> --broadcast
contract SubmitProof is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address verifierAddress = vm.envAddress("RORSCHACH_VERIFIER");

        console2.log("=== Submitting Proof ===");
        console2.log("Submitter:", vm.addr(deployerPrivateKey));
        console2.log("Verifier:", verifierAddress);

        // Load proof files
        string memory root = vm.projectRoot();
        string memory sealFile = vm.envOr("SEAL_FILE", string("test_groth16.seal"));
        string memory journalFile = vm.envOr("JOURNAL_FILE", string("test_groth16.journal"));

        bytes memory seal = vm.readFileBinary(
            string.concat(root, "/../../", sealFile)
        );
        bytes memory journal = vm.readFileBinary(
            string.concat(root, "/../../", journalFile)
        );

        console2.log("Seal size:", seal.length, "bytes");
        console2.log("Journal size:", journal.length, "bytes");

        // Decode to show what we're verifying
        (address ethAddress, uint64 walks, uint64 steps, bytes memory binaryImage) =
            abi.decode(journal, (address, uint64, uint64, bytes));

        console2.log("\n=== Proof Data ===");
        console2.log("Address:", ethAddress);
        console2.log("Walks:", walks);
        console2.log("Steps:", steps);
        console2.log("Binary image size:", binaryImage.length, "bytes");

        bytes32 imageHash = keccak256(binaryImage);
        console2.log("Image hash:", vm.toString(imageHash));

        // Estimate gas
        RorschachVerifier verifier = RorschachVerifier(verifierAddress);

        console2.log("\n=== Submitting to blockchain ===");
        vm.startBroadcast(deployerPrivateKey);

        bool success = verifier.verifyImage(seal, journal);

        vm.stopBroadcast();

        require(success, "Proof verification failed");

        console2.log("\n=== Success! ===");
        console2.log("[OK] Proof verified on-chain!");
        console2.log("[OK] Image hash:", vm.toString(imageHash));
        console2.log("[OK] Address:", ethAddress);

        // Check verification
        bool isVerified = verifier.isVerified(imageHash);
        console2.log("[OK] Verification status:", isVerified);

        bytes32[] memory images = verifier.getAddressImages(ethAddress);
        console2.log("[OK] Total images for address:", images.length);
    }
}
