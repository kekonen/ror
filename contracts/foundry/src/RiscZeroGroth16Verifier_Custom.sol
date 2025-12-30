// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.9;

import {RiscZeroGroth16Verifier} from "risc0-ethereum/contracts/src/groth16/RiscZeroGroth16Verifier.sol";

/// @notice Custom Groth16 verifier that uses the selector from our Docker prover
/// @dev This verifier uses the same code as RiscZeroGroth16Verifier but with a custom selector
///      to match the Docker image risczero/risc0-groth16-prover:v2025-04-03.1
contract RiscZeroGroth16Verifier_Custom is RiscZeroGroth16Verifier {
    /// @notice Selector from our Docker-generated proof
    /// @dev This is hardcoded to match the Docker image output
    bytes4 public constant CUSTOM_SELECTOR = 0x2f80f8d0;

    constructor(bytes32 control_root, bytes32 bn254_control_id)
        RiscZeroGroth16Verifier(control_root, bn254_control_id)
    {
        // The parent constructor computes SELECTOR from the verification key
        // We'll override it in the verify functions
    }

    /// @notice Override to use our custom selector instead of the computed one
    function _verifyIntegrity(bytes calldata seal, bytes32 claimDigest) internal view override {
        // Check that the seal has a matching selector
        if (CUSTOM_SELECTOR != bytes4(seal[:4])) {
            revert SelectorMismatch({received: bytes4(seal[:4]), expected: CUSTOM_SELECTOR});
        }

        // Decode the seal
        Seal memory _seal = abi.decode(seal[4:], (Seal));

        // Hash the claim to get the 5 public signals required by the Groth16 verifier.
        bytes32 claimDigestTruncated = claimDigest & DIGEST_BUNDLE_USAGE_MASK;
        uint256 h = uint256(sha256(abi.encodePacked(claimDigestTruncated))) & ((1 << 253) - 1);
        uint256[5] memory publicSignals = [
            h >> 224,
            (h >> 192) & ((1 << 32) - 1),
            (h >> 160) & ((1 << 32) - 1),
            (h >> 128) & ((1 << 32) - 1),
            h & ((1 << 128) - 1)
        ];

        // Verify the groth16 proof
        bool verified = GROTH16_VERIFIER.verifyProof(_seal.a, _seal.b, _seal.c, publicSignals);
        if (!verified) {
            revert VerificationFailed();
        }
    }
}
