// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @notice Minimal trust-anchor contract. Stores the current Merkle root
/// published by the issuer. Performs NO proof verification and NO
/// credential logic — that is intentional (see architecture: Ethereum
/// layer is root storage only).
contract CredentialRegistry {
    address public immutable issuer;
    bytes32 public currentRoot;

    event RootPublished(bytes32 indexed root, uint256 timestamp);

    error NotIssuer();

    modifier onlyIssuer() {
        if (msg.sender != issuer) revert NotIssuer();
        _;
    }

    constructor() {
        issuer = msg.sender;
    }

    function publishRoot(bytes32 root) external onlyIssuer {
        currentRoot = root;
        emit RootPublished(root, block.timestamp);
    }

    function getRoot() external view returns (bytes32) {
        return currentRoot;
    }
}
