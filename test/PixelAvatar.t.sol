// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {PixelAvatar, PixelAvatarRenderer} from "../src/PixelAvatar.sol";

contract PixelAvatarTest is Test {
    PixelAvatarRenderer internal renderer;
    string internal json;

    function setUp() public {
        renderer = new PixelAvatarRenderer();
        json = vm.readFile("test/fixtures/avatars.json");
    }

    /// The SVG rendered on-chain equals the one produced by web/index.html (and the Rust crate).
    function test_SvgMatchesWebFixture() public view {
        uint256 count = vm.parseJsonUint(json, ".count");
        for (uint256 i; i < count; ++i) {
            string memory base = string.concat(".items[", vm.toString(i), "]");
            address account = vm.parseJsonAddress(json, string.concat(base, ".address"));
            string memory expected = vm.parseJsonString(json, string.concat(base, ".pixelid"));
            assertEq(renderer.svgOf(account), expected);
        }
    }

    function test_DataUriIsBase64OfSvg() public view {
        address account = address(0xdEaD);
        string memory svg = renderer.svgOf(account);
        assertEq(renderer.dataURIOf(account), string.concat("data:image/svg+xml;base64,", vm.toBase64(bytes(svg))));
    }

    function test_TokenJsonIsValid() public view {
        string memory meta = renderer.tokenJSON(address(0xdEaD));
        assertEq(vm.parseJsonString(meta, ".name"), "PixelID 0x000000000000000000000000000000000000dead");
        assertEq(vm.parseJsonString(meta, ".image"), renderer.dataURIOf(address(0xdEaD)));
    }

    function test_PaletteIsHsl() public view {
        string[3] memory colors = renderer.paletteOf(address(0xBEEF));
        for (uint256 i; i < 3; ++i) {
            assertEq(bytes(colors[i])[0], bytes1("h"));
        }
    }

    function testFuzz_GridIsMirroredAndBounded(address account) public view {
        uint8[64] memory cells = renderer.gridOf(account);
        for (uint256 y; y < 8; ++y) {
            for (uint256 x; x < 8; ++x) {
                assertLe(cells[y * 8 + x], 2);
                assertEq(cells[y * 8 + x], cells[y * 8 + 7 - x]);
            }
        }
    }

    function testFuzz_Deterministic(address account) public view {
        assertEq(keccak256(bytes(renderer.svgOf(account))), keccak256(bytes(renderer.svgOf(account))));
    }

    function test_GasSnapshot() public {
        vm.startSnapshotGas("svgOf");
        renderer.svgOf(address(0xdEaD));
        uint256 used = vm.stopSnapshotGas();
        assertLt(used, 400_000);
    }
}
