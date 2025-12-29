// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script} from "forge-std/Script.sol";
import {console2} from "forge-std/console2.sol";
import {RorschachVerifier} from "../src/RorschachVerifier.sol";
import {IRiscZeroVerifier} from "../src/IRiscZeroVerifier.sol";

/// @notice Deployment script for RorschachVerifier
/// @dev Run with: forge script script/Deploy.s.sol --rpc-url <network> --broadcast
contract Deploy is Script {
    // risc0 Groth16 verifier addresses
    // Source: https://dev.risczero.com/api/blockchain-integration/contracts/verifier
    address constant RISC0_VERIFIER_MAINNET = 0x8EaB2D97Dfce405A1692a21b3ff3A172d593D319;
    address constant RISC0_VERIFIER_SEPOLIA = 0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187;
    address constant RISC0_VERIFIER_HOLESKY = 0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187; // Same as Sepolia

    function run() external {
        // Get deployer from private key env var
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        console2.log("=== Deploying RorschachVerifier ===");
        console2.log("Deployer:", deployer);
        console2.log("Chain ID:", block.chainid);
        console2.log("Balance:", deployer.balance / 1e18, "ETH");

        // Select appropriate risc0 verifier based on chain
        address risc0Verifier = getRisc0Verifier();
        console2.log("Using risc0 verifier:", risc0Verifier);

        // Check verifier exists
        uint256 codeSize;
        assembly {
            codeSize := extcodesize(risc0Verifier)
        }
        require(codeSize > 0, "risc0 verifier not found on this network");
        console2.log("Verifier code size:", codeSize, "bytes");

        // Deploy
        vm.startBroadcast(deployerPrivateKey);

        RorschachVerifier verifier = new RorschachVerifier(
            IRiscZeroVerifier(risc0Verifier)
        );

        vm.stopBroadcast();

        // Log deployment info
        console2.log("\n=== Deployment Successful! ===");
        console2.log("RorschachVerifier:", address(verifier));
        console2.log("risc0 Verifier:", risc0Verifier);
        console2.log("ImageID (update required):", vm.toString(verifier.imageId()));

        // Save to file
        string memory deploymentInfo = string.concat(
            "# Deployment Info\n\n",
            "**Network**: ", getNetworkName(), "\n",
            "**Chain ID**: ", vm.toString(block.chainid), "\n",
            "**Deployer**: ", vm.toString(deployer), "\n\n",
            "## Contracts\n\n",
            "- **RorschachVerifier**: `", vm.toString(address(verifier)), "`\n",
            "- **risc0 Verifier**: `", vm.toString(risc0Verifier), "`\n\n",
            "## Next Steps\n\n",
            "1. Update `ImageID.sol` with correct GUEST_ID\n",
            "2. Verify contract on block explorer\n",
            "3. Test with: `forge script script/SubmitProof.s.sol --rpc-url <network> --broadcast`\n"
        );

        vm.writeFile("deployment.md", deploymentInfo);
        console2.log("\nDeployment info saved to deployment.md");

        // Print verification command
        console2.log("\n=== Verify on Etherscan ===");
        console2.log("forge verify-contract", address(verifier), "src/RorschachVerifier.sol:RorschachVerifier");
        console2.log("  --chain-id", block.chainid);
        console2.log("  --constructor-args $(cast abi-encode 'constructor(address)' ", risc0Verifier, ")");
        console2.log("  --watch");
    }

    function getRisc0Verifier() internal view returns (address) {
        if (block.chainid == 1) {
            // Ethereum Mainnet
            return RISC0_VERIFIER_MAINNET;
        } else if (block.chainid == 11155111) {
            // Sepolia
            return RISC0_VERIFIER_SEPOLIA;
        } else if (block.chainid == 17000) {
            // Holesky
            return RISC0_VERIFIER_HOLESKY;
        } else if (block.chainid == 31337) {
            // Anvil/Hardhat - use env var or mainnet as default
            address customVerifier = vm.envOr("RISC0_VERIFIER_ADDRESS", RISC0_VERIFIER_MAINNET);
            return customVerifier;
        } else {
            // Try environment variable
            return vm.envAddress("RISC0_VERIFIER_ADDRESS");
        }
    }

    function getNetworkName() internal view returns (string memory) {
        if (block.chainid == 1) return "Ethereum Mainnet";
        if (block.chainid == 11155111) return "Sepolia Testnet";
        if (block.chainid == 17000) return "Holesky Testnet";
        if (block.chainid == 31337) return "Local (Anvil/Hardhat)";
        return string.concat("Unknown (", vm.toString(block.chainid), ")");
    }
}
