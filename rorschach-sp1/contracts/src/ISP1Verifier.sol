// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title ISP1Verifier
/// @notice Interface for SP1 Groth16 verifier
/// @dev SP1 has pre-deployed verifiers on major networks
///      See https://docs.succinct.xyz/verification/on-chain/getting-started
interface ISP1Verifier {
    /// @notice Verifies a proof
    /// @param programVKey The verification key for the program (32 bytes)
    /// @param publicValues The public values encoded as bytes
    /// @param proofBytes The proof bytes (Groth16 proof, ~260 bytes)
    function verifyProof(
        bytes32 programVKey,
        bytes calldata publicValues,
        bytes calldata proofBytes
    ) external view;
}
