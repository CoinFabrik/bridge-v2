// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/Pausable.sol";

contract GlitchBridge is Ownable, Pausable {
    using SafeERC20 for IERC20;

    /* ========== STATE VARIABLES ========== */
    IERC20 public glitchToken;
    address public immutable DESTINY_ADDRESS;

    uint256 public minAmount;
    uint256 public maxAmount;

    /* ========== EVENTS ========== */
    event TransferToGlitch(address indexed from_eth, string _glitchAddress, uint256 _amount);

    /* ========== CONSTRUCTOR ========== */
    constructor(address _tokenAddress) {
        glitchToken = IERC20(_tokenAddress);
        minAmount = 100 ether;
        maxAmount = 40_000 ether;
        DESTINY_ADDRESS = 0x1daC534E857051381201c160CFfc66c61E2316Ed;
    }

    /* ========== MUTATIVE FUNCTIONS ========== */
    function transferToGlitch(string memory _glitchAddress, uint256 _amount)
        external
        whenNotPaused
        validateLimits(_amount)
    {
        glitchToken.transferFrom(msg.sender, DESTINY_ADDRESS, _amount);

        emit TransferToGlitch(msg.sender, _glitchAddress, _amount);
    }

    function setMinAmount(uint256 _newAmount) external onlyOwner {
        minAmount = _newAmount;
    }

    function setMaxAmount(uint256 _newAmount) external onlyOwner {
        maxAmount = _newAmount;
    }

    function pause() external onlyOwner {
        _pause();
    }

    function unpause() external onlyOwner {
        _unpause();
    }

    /* ========== MODIFIERS ========== */
    modifier validateLimits(uint256 _amount) {
        require(_amount >= minAmount && _amount <= maxAmount, "Invalid amount!");
        _;
    }
}
