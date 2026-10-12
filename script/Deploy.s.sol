// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script, console2} from "forge-std/Script.sol";
import {PixelAvatarRenderer} from "../src/PixelAvatar.sol";

/// forge script script/Deploy.s.sol --rpc-url $RPC_URL --account deployer --broadcast
contract Deploy is Script {
    function run() external returns (PixelAvatarRenderer renderer) {
        vm.startBroadcast();
        renderer = new PixelAvatarRenderer();
        vm.stopBroadcast();
        console2.log("PixelAvatarRenderer", address(renderer));
    }
}
