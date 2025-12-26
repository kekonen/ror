// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./ImageID.sol"; // Auto-generated from guest build

/// @title RorschachVerifier
/// @notice Verifies that a Rorschach image was generated from a private key
/// @dev Uses risc0 Groth16 verifier for on-chain proof verification
contract RorschachVerifier {
    /// @notice The risc0 Groth16 verifier contract
    IRiscZeroVerifier public immutable verifier;

    /// @notice Image ID from the guest program (ensures correct program was executed)
    bytes32 public immutable imageId;

    /// @notice Emitted when an image is verified
    /// @param imageHash Keccak256 hash of the generated image
    /// @param ethAddress Ethereum address derived from the private key
    /// @param walks Number of walks used in generation
    /// @param steps Number of steps used in generation
    event ImageVerified(
        bytes32 indexed imageHash,
        address indexed ethAddress,
        uint64 walks,
        uint64 steps
    );

    /// @notice Mapping of verified images (imageHash => true)
    mapping(bytes32 => bool) public verifiedImages;

    /// @notice Mapping of addresses to their verified images
    mapping(address => bytes32[]) public addressImages;

    constructor(IRiscZeroVerifier _verifier) {
        verifier = _verifier;
        imageId = ImageID.GENERATE_IMAGE_ID;
    }

    /// @notice Verify a Groth16 proof of image generation
    /// @param seal The Groth16 proof seal
    /// @param journal The public outputs from the guest program
    /// @return True if verification succeeds
    function verifyImage(bytes calldata seal, bytes calldata journal)
        external
        returns (bool)
    {
        // Verify the Groth16 proof
        verifier.verify(seal, imageId, sha256(journal));

        // Decode the journal (public outputs)
        (
            address ethAddress,
            uint64 walks,
            uint64 steps,
            bytes memory imageBytes
        ) = abi.decode(journal, (address, uint64, uint64, bytes));

        // Hash the image for storage
        bytes32 imageHash = keccak256(imageBytes);

        // Mark as verified
        verifiedImages[imageHash] = true;
        addressImages[ethAddress].push(imageHash);

        emit ImageVerified(imageHash, ethAddress, walks, steps);

        return true;
    }

    /// @notice Check if an image has been verified
    /// @param imageHash Keccak256 hash of the image bytes
    function isVerified(bytes32 imageHash) external view returns (bool) {
        return verifiedImages[imageHash];
    }

    /// @notice Get all verified images for an address
    /// @param ethAddress The Ethereum address
    function getAddressImages(address ethAddress)
        external
        view
        returns (bytes32[] memory)
    {
        return addressImages[ethAddress];
    }

    /// @notice Verify ownership: prove you control the private key for this address
    /// @param seal The Groth16 proof seal
    /// @param journal The public outputs
    /// @param claimedAddress The address you claim to own
    function verifyOwnership(
        bytes calldata seal,
        bytes calldata journal,
        address claimedAddress
    ) external view returns (bool) {
        // Verify the proof
        verifier.verify(seal, imageId, sha256(journal));

        // Decode journal
        (address ethAddress, , , ) = abi.decode(
            journal,
            (address, uint64, uint64, bytes)
        );

        // Check if the derived address matches the claimed address
        return ethAddress == claimedAddress;
    }
}
