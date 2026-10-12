// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title PixelAvatar
/// @notice Fully on-chain, deterministic 8x8 pixel avatar for any address (blockie-style identicon).
///         The exact same algorithm runs in ../index.html ("PixelID" style), so the browser preview and
///         the chain always agree byte for byte.
///
/// Algorithm "PixelID v1"
///   h = keccak256(abi.encodePacked(account))      -> colours
///   c = keccak256(abi.encodePacked(h))            -> pixels
///   hue[i]  = uint16(h[2i], h[2i+1]) % 360          i = 0 fg, 1 bg, 2 spot
///   sat[i]  = 40 + h[6+i] % 50
///   lightness fg = 35 + h[9] % 25, bg = 82 + h[10] % 12, spot = 50 + h[11] % 25
///   cell (x<4, y<8) = c[y*4+x] % 100 : <40 background, <82 foreground, else spot
///   columns are mirrored (x and 7-x) for a symmetric, face-like shape.
library PixelAvatar {
    uint256 internal constant GRID = 8;

    /// @notice Raw 8x8 grid, row-major, values 0 (bg) / 1 (fg) / 2 (spot).
    function grid(address account) internal pure returns (uint8[64] memory cells) {
        bytes32 c = keccak256(abi.encodePacked(keccak256(abi.encodePacked(account))));
        for (uint256 y = 0; y < GRID; ++y) {
            for (uint256 x = 0; x < GRID / 2; ++x) {
                uint8 v = uint8(c[y * 4 + x]) % 100;
                uint8 cell = v < 40 ? 0 : (v < 82 ? 1 : 2);
                cells[y * GRID + x] = cell;
                cells[y * GRID + (GRID - 1 - x)] = cell;
            }
        }
    }

    /// @notice The three palette colours as CSS `hsl()` strings: [fg, bg, spot].
    function palette(address account) internal pure returns (string[3] memory colors) {
        bytes32 h = keccak256(abi.encodePacked(account));
        uint256[3] memory light =
            [35 + uint256(uint8(h[9])) % 25, 82 + uint256(uint8(h[10])) % 12, 50 + uint256(uint8(h[11])) % 25];
        for (uint256 i = 0; i < 3; ++i) {
            uint256 hue = ((uint256(uint8(h[2 * i])) << 8) | uint256(uint8(h[2 * i + 1]))) % 360;
            uint256 sat = 40 + uint256(uint8(h[6 + i])) % 50;
            colors[i] = string.concat("hsl(", _u(hue), ",", _u(sat), "%,", _u(light[i]), "%)");
        }
    }

    /// @notice Standalone SVG markup (8x8 viewBox, crisp pixels, scales to any size).
    function svg(address account) internal pure returns (string memory out) {
        string[3] memory col = palette(account);
        uint8[64] memory cells = grid(account);
        out = string.concat(
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8" shape-rendering="crispEdges">',
            '<rect width="8" height="8" fill="',
            col[1],
            '"/>'
        );
        for (uint256 i = 0; i < 64; ++i) {
            uint8 v = cells[i];
            if (v == 0) continue;
            out = string.concat(
                out,
                '<rect x="',
                _u(i % GRID),
                '" y="',
                _u(i / GRID),
                '" width="1" height="1" fill="',
                v == 1 ? col[0] : col[2],
                '"/>'
            );
        }
        out = string.concat(out, "</svg>");
    }

    /// @notice `data:image/svg+xml;base64,...` ready for an <img src> or ERC-721 `image` field.
    function dataURI(address account) internal pure returns (string memory) {
        return string.concat("data:image/svg+xml;base64,", _base64(bytes(svg(account))));
    }

    // ------------------------------------------------------------------ utils

    function _u(uint256 v) private pure returns (string memory) {
        if (v == 0) return "0";
        uint256 len;
        for (uint256 t = v; t != 0; t /= 10) {
            ++len;
        }
        bytes memory b = new bytes(len);
        while (v != 0) {
            b[--len] = bytes1(uint8(48 + (v % 10)));
            v /= 10;
        }
        return string(b);
    }

    function _base64(bytes memory data) private pure returns (string memory) {
        if (data.length == 0) return "";
        bytes memory table = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        bytes memory out = new bytes(4 * ((data.length + 2) / 3));
        uint256 j;
        for (uint256 i = 0; i < data.length; i += 3) {
            uint256 a = uint8(data[i]);
            uint256 b = i + 1 < data.length ? uint8(data[i + 1]) : 0;
            uint256 c = i + 2 < data.length ? uint8(data[i + 2]) : 0;
            uint256 triple = (a << 16) | (b << 8) | c;
            out[j++] = table[(triple >> 18) & 63];
            out[j++] = table[(triple >> 12) & 63];
            out[j++] = i + 1 < data.length ? table[(triple >> 6) & 63] : bytes1("=");
            out[j++] = i + 2 < data.length ? table[triple & 63] : bytes1("=");
        }
        return string(out);
    }
}

/// @title PixelAvatarRenderer
/// @notice Thin deployable wrapper so dapps / other contracts can call the library over the network.
contract PixelAvatarRenderer {
    function svgOf(address account) external pure returns (string memory) {
        return PixelAvatar.svg(account);
    }

    function dataURIOf(address account) external pure returns (string memory) {
        return PixelAvatar.dataURI(account);
    }

    function gridOf(address account) external pure returns (uint8[64] memory) {
        return PixelAvatar.grid(account);
    }

    function paletteOf(address account) external pure returns (string[3] memory) {
        return PixelAvatar.palette(account);
    }

    /// @notice ERC-721 style metadata, e.g. for a soulbound "profile pic" token keyed by owner address.
    function tokenJSON(address account) external pure returns (string memory) {
        return string.concat(
            '{"name":"PixelID ',
            _hex(account),
            '","description":"Deterministic on-chain pixel avatar.","image":"',
            PixelAvatar.dataURI(account),
            '"}'
        );
    }

    function _hex(address account) private pure returns (string memory) {
        bytes memory alphabet = "0123456789abcdef";
        bytes20 v = bytes20(account);
        bytes memory s = new bytes(42);
        s[0] = "0";
        s[1] = "x";
        for (uint256 i = 0; i < 20; ++i) {
            s[2 + i * 2] = alphabet[uint8(v[i]) >> 4];
            s[3 + i * 2] = alphabet[uint8(v[i]) & 0x0f];
        }
        return string(s);
    }
}
