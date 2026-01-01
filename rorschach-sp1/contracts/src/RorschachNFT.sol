// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ERC721} from "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import {ERC721URIStorage} from "@openzeppelin/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Base64} from "@openzeppelin/contracts/utils/Base64.sol";
import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";
import {ISP1Verifier} from "./ISP1Verifier.sol";

/// @title RorschachNFT
/// @notice Zero-knowledge Rorschach pattern NFTs
/// @dev Mints NFTs based on ZK proofs of private key knowledge
/// @dev Private keys are never revealed on-chain, only ZK proofs
contract RorschachNFT is ERC721, ERC721URIStorage, Ownable {
    using Strings for uint256;

    /// @notice The SP1 verifier contract
    ISP1Verifier public immutable verifier;

    /// @notice The program verification key
    bytes32 public immutable programVKey;

    /// @notice Mapping of image hashes to claim status
    mapping(bytes32 => bool) public claimed;

    /// @notice Mapping of token ID to image data
    mapping(uint256 => ImageData) public tokenImages;

    /// @notice Structure to store image data on-chain
    struct ImageData {
        address derivedAddress;
        uint64 walks;
        uint64 steps;
        bytes32 imageHash;
        bytes imageData; // 256 bytes packed binary
    }

    /// @notice Emitted when a Rorschach is claimed
    event RorschachClaimed(
        address indexed claimer,
        uint256 indexed tokenId,
        address derivedAddress,
        uint64 walks,
        uint64 steps,
        bytes32 imageHash
    );

    error AlreadyClaimed(bytes32 imageHash);
    error InvalidProof();
    error InvalidPublicValues();

    constructor(
        address _verifier,
        bytes32 _programVKey
    ) ERC721("Rorschach", "ROR") Ownable(msg.sender) {
        verifier = ISP1Verifier(_verifier);
        programVKey = _programVKey;
    }

    /// @notice Claim a Rorschach NFT by providing a valid ZK proof
    /// @param proofBytes The Groth16 proof bytes
    /// @param publicValues The encoded public values from the proof
    /// @dev Public values layout: address (20) + walks (8) + steps (8) + imageHash (32) + imageData (256) = 324 bytes
    function claim(
        bytes calldata proofBytes,
        bytes calldata publicValues
    ) external returns (uint256) {
        // Verify the ZK proof
        try verifier.verifyProof(programVKey, publicValues, proofBytes) {
            // Proof is valid
        } catch {
            revert InvalidProof();
        }

        // Decode public values
        // Layout: address (20) + walks (8) + steps (8) + imageHash (32) + imageData (256)
        if (publicValues.length != 324) {
            revert InvalidPublicValues();
        }

        address derivedAddress;
        uint64 walks;
        uint64 steps;
        bytes32 imageHash;
        bytes memory imageData = new bytes(256);

        assembly {
            // Load address (20 bytes, right-padded in a 32-byte word)
            derivedAddress := shr(96, calldataload(publicValues.offset))

            // Load walks (8 bytes as uint64)
            walks := calldataload(add(publicValues.offset, 20))
            walks := shr(192, walks) // Shift to get uint64

            // Load steps
            steps := calldataload(add(publicValues.offset, 28))
            steps := shr(192, steps)

            // Load imageHash
            imageHash := calldataload(add(publicValues.offset, 36))
        }

        // Copy image data
        for (uint256 i = 0; i < 256; i++) {
            imageData[i] = publicValues[68 + i];
        }

        // Check not already claimed
        if (claimed[imageHash]) {
            revert AlreadyClaimed(imageHash);
        }
        claimed[imageHash] = true;

        // Generate token ID from image hash
        uint256 tokenId = uint256(imageHash);

        // Store image data
        tokenImages[tokenId] = ImageData({
            derivedAddress: derivedAddress,
            walks: walks,
            steps: steps,
            imageHash: imageHash,
            imageData: imageData
        });

        // Mint NFT
        _safeMint(msg.sender, tokenId);

        emit RorschachClaimed(
            msg.sender,
            tokenId,
            derivedAddress,
            walks,
            steps,
            imageHash
        );

        return tokenId;
    }

    /// @notice Get image data for a token
    function getImageData(uint256 tokenId) external view returns (ImageData memory) {
        return tokenImages[tokenId];
    }

    /// @notice Generate on-chain SVG for the NFT
    function tokenURI(uint256 tokenId) public view override(ERC721, ERC721URIStorage) returns (string memory) {
        ImageData memory data = tokenImages[tokenId];

        // Generate SVG from binary data
        string memory svg = _generateSVG(data.imageData);

        // Create metadata JSON
        string memory json = string(abi.encodePacked(
            '{"name": "Rorschach #',
            tokenId.toString(),
            '", "description": "A cryptographically unique Rorschach pattern proven via zero-knowledge proof.", ',
            '"attributes": [',
            '{"trait_type": "Walks", "value": ', uint256(data.walks).toString(), '},',
            '{"trait_type": "Steps", "value": ', uint256(data.steps).toString(), '}',
            '], "image": "data:image/svg+xml;base64,',
            Base64.encode(bytes(svg)),
            '"}'
        ));

        return string(abi.encodePacked(
            "data:application/json;base64,",
            Base64.encode(bytes(json))
        ));
    }

    /// @notice Generate SVG from packed binary image data
    /// @dev Creates a 64x64 SVG with mirrored symmetry
    function _generateSVG(bytes memory imageData) internal pure returns (string memory) {
        string memory pixels = "";

        // Each byte contains 8 pixels
        // 32x64 half-image, mirrored for 64x64 full image
        for (uint256 y = 0; y < 64; y++) {
            for (uint256 x = 0; x < 32; x++) {
                uint256 bitIndex = y * 32 + x;
                uint256 byteIndex = bitIndex / 8;
                uint256 bitOffset = bitIndex % 8;

                bool isSet = (uint8(imageData[byteIndex]) & (1 << bitOffset)) != 0;

                if (isSet) {
                    // Add pixel for left half
                    pixels = string(abi.encodePacked(
                        pixels,
                        '<rect x="', x.toString(), '" y="', y.toString(),
                        '" width="1" height="1" fill="#FFD966"/>'
                    ));
                    // Add mirrored pixel for right half
                    pixels = string(abi.encodePacked(
                        pixels,
                        '<rect x="', (63 - x).toString(), '" y="', y.toString(),
                        '" width="1" height="1" fill="#FFD966"/>'
                    ));
                }
            }
        }

        return string(abi.encodePacked(
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">',
            '<rect width="64" height="64" fill="#FF0081"/>',
            pixels,
            '</svg>'
        ));
    }

    // Required overrides
    function supportsInterface(bytes4 interfaceId)
        public
        view
        override(ERC721, ERC721URIStorage)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }
}
