// SPDX-License-Identifier: MIT
pragma solidity >=0.6.2;

import "@openzeppelin/contracts/math/SafeMath.sol";
import "./Decoder.sol";
import "./Application.sol";

contract ETHApp is Application {
    using SafeMath for uint256;
    using Decoder for bytes;

    uint64 MESSAGE_LENGTH = 84;

    uint256 public nonce;
    uint256 public totalETH;

    event Transfer(address _sender, bytes32 _recipient, uint256 _amount);
    event Unlock(bytes _sender, address _recipient, uint256 _amount);

    constructor() public {
        nonce = 0;
    }

    function sendETH(bytes32 _recipient)
        public
        payable
    {
        require(msg.value > 0, "Value of transaction must be positive");

        // Increment locked Ethereum counter by this amount
        totalETH = totalETH.add(msg.value);
        // Increment global nonce
        nonce = nonce.add(1);

        emit Transfer(msg.sender, _recipient, msg.value);
    }

    function handle(bytes memory _data)
        public
        override
    {
        require(_data.length == MESSAGE_LENGTH, "Data must contain 84 bytes for a successful decoding");

        // Decode sender bytes
        bytes memory sender = _data.slice(0, 32);
        // Decode recipient address
        address payable recipient = _data.sliceAddress(32);
        // Deocde amount int256
        bytes memory amountBytes = _data.slice(32 + 20, 32);
        uint256 amount = amountBytes.decodeUint256();

        unlockETH(recipient, amount);
        emit Unlock(sender, recipient, amount);
    }

    function unlockETH(address payable _recipient, uint256 _amount)
        internal
    {
        require(_amount > 0, "Must unlock a positive amount");
        require(totalETH > _amount, "ETH token balances insufficient to fulfill the unlock request");

        totalETH = totalETH.sub(_amount);
        _recipient.transfer(_amount);
    }
}
