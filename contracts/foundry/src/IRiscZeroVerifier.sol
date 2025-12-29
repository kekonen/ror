// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

/// @notice Interface for risc0 verifier contracts
interface IRiscZeroVerifier {
    /// @notice Verify a proof with journal hash
    /// @param seal The encoded cryptographic proof (Groth16)
    /// @param imageId The identifier for the guest program
    /// @param journalDigest The SHA-256 digest of the journal
    function verify(
        bytes calldata seal,
        bytes32 imageId,
        bytes32 journalDigest
    ) external view;
}
