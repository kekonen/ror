// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "./IRiscZeroVerifier.sol";
import {ImageID} from "./ImageID.sol";

/// @title RorschachVerifier
/// @notice Verifies Rorschach image proofs using risc0 Groth16
/// @dev Optimized for binary image format (256 bytes)
contract RorschachVerifier {
    IRiscZeroVerifier public immutable verifier;
    bytes32 public immutable imageId;

    struct ImageData {
        address ethAddress;
        uint64 walks;
        uint64 steps;
        bytes binaryImage;  // 256 bytes packed binary
    }

    event ImageVerified(
        bytes32 indexed imageHash,
        address indexed ethAddress,
        uint64 walks,
        uint64 steps,
        uint256 timestamp
    );

    mapping(bytes32 => bool) public verifiedImages;
    mapping(address => bytes32[]) public addressImages;

    error InvalidProof();
    error InvalidImageSize();
    error AlreadyVerified();

    constructor(IRiscZeroVerifier _verifier) {
        verifier = _verifier;
        imageId = ImageID.GUEST_ID;
    }

    /// @notice Verify a Groth16 proof of Rorschach generation
    /// @param seal The Groth16 proof seal from risc0
    /// @param journal The public outputs (ABI-encoded ImageData)
    function verifyImage(bytes calldata seal, bytes calldata journal)
        external
        returns (bool)
    {
        // Verify Groth16 proof
        verifier.verify(seal, imageId, sha256(journal));

        // Decode journal data
        (address ethAddress, uint64 walks, uint64 steps, bytes memory binaryImage) =
            abi.decode(journal, (address, uint64, uint64, bytes));

        // Validate binary image size (must be exactly 256 bytes)
        if (binaryImage.length != 256) revert InvalidImageSize();

        // Calculate image hash
        bytes32 imageHash = keccak256(binaryImage);

        // Check not already verified
        if (verifiedImages[imageHash]) revert AlreadyVerified();

        // Store verification
        verifiedImages[imageHash] = true;
        addressImages[ethAddress].push(imageHash);

        emit ImageVerified(imageHash, ethAddress, walks, steps, block.timestamp);

        return true;
    }

    /// @notice Verify ownership without storing (view function)
    /// @param seal The Groth16 proof seal
    /// @param journal The public outputs
    /// @param claimedAddress The address to verify
    function verifyOwnership(
        bytes calldata seal,
        bytes calldata journal,
        address claimedAddress
    ) external view returns (bool) {
        verifier.verify(seal, imageId, sha256(journal));

        (address ethAddress, , , ) = abi.decode(
            journal,
            (address, uint64, uint64, bytes)
        );

        return ethAddress == claimedAddress;
    }

    /// @notice Get all verified images for an address
    function getAddressImages(address addr)
        external
        view
        returns (bytes32[] memory)
    {
        return addressImages[addr];
    }

    /// @notice Check if an image has been verified
    function isVerified(bytes32 imageHash) external view returns (bool) {
        return verifiedImages[imageHash];
    }
}
