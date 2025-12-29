// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title ImageID
/// @notice Guest program identifier
/// @dev This will be updated with the actual GUEST_ID from the build
library ImageID {
    /// @notice Image ID from methods/guest build
    /// @dev This is the pre-state digest from the receipt claim
    /// Generated from: [279451329, 404017501, 1266723334, 1839516920, 3206915956, 107017358, 1632265199, 2197116899]
    /// Verified from receipt: test_groth16_new.groth16.proof
    bytes32 public constant GUEST_ID = 0xc116a8105dd1141806aa804bf8cca46d74a725bf8ef46006ef634a61e357f582;
}
