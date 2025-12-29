// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.9;

/// @title ControlID for RISC Zero v3.0.4
/// @notice Control IDs extracted from risc0-circuit-recursion v4.0.3
/// @dev These values are from risc0-zkvm v3.0.4 / risc0-circuit-recursion v4.0.3
library ControlID_v3 {
    /// @notice Control root hash for risc0 Groth16
    /// @dev From risc0-ethereum v1.2.0 (matches Docker image v2025-04-03.1)
    bytes32 public constant CONTROL_ROOT = hex"8cdad9242664be3112aba377c5425a4df735eb1c6966472b561d2855932c0469";
    
    /// @notice BN254 identity control ID for Groth16 verification
    /// @dev From risc0_circuit_recursion::control_id::BN254_IDENTITY_CONTROL_ID
    /// NOTE: This has the opposite byte order to the value in the risc0 repository (as per risc0-ethereum convention)
    bytes32 public constant BN254_CONTROL_ID = hex"04644e66d300eb7fb45c9726bb53c793dda407a62e960161b6483b5c14657ac0";
}
