// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";

contract GlitchToken is ERC20Burnable {
    constructor() ERC20("Glitch", "GLCH") {
        _mint(msg.sender, 21_000_000 * (10 ** uint256(decimals())));
    }
}