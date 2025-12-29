// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import {Script} from "forge-std/Script.sol";
import {console2} from "forge-std/console2.sol";
import {IRiscZeroVerifier} from "../src/IRiscZeroVerifier.sol";
import {RiscZeroGroth16Verifier} from "risc0-ethereum/contracts/src/groth16/RiscZeroGroth16Verifier.sol";
import {ControlID} from "risc0-ethereum/contracts/src/groth16/ControlID.sol";

/// @notice Deployment script for the RISC Zero Groth16 verifier
/// @dev This deploys the verifier contract that matches our risc0 v3.0 proofs
contract DeployRiscZeroVerifier is Script {
    function run() external returns (address) {
        uint256 deployerKey = vm.envUint("PRIVATE_KEY");

        vm.startBroadcast(deployerKey);

        // Deploy the Groth16 verifier with risc0-ethereum v1.2.0 control IDs
        RiscZeroGroth16Verifier verifierImpl = new RiscZeroGroth16Verifier(
            ControlID.CONTROL_ROOT,
            ControlID.BN254_CONTROL_ID
        );
        IRiscZeroVerifier verifier = IRiscZeroVerifier(address(verifierImpl));

        console2.log("\n=== RISC Zero Groth16 Verifier Deployed (v1.2.0) ===");
        console2.log("Address:", address(verifier));
        console2.log("CONTROL_ROOT:", vm.toString(ControlID.CONTROL_ROOT));
        console2.log("BN254_CONTROL_ID:", vm.toString(ControlID.BN254_CONTROL_ID));
        
        vm.stopBroadcast();

        return address(verifier);
    }
}
