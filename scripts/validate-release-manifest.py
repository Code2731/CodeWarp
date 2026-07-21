#!/usr/bin/env python3
"""Validate the source inputs required for a portable CodeWarp archive."""

# /// script
# requires-python = ">=3.11"
# ///

from __future__ import annotations

import argparse
import re
import stat
import sys
import tomllib
from pathlib import Path

REQUIRED_INPUTS = (
    "README.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "assets/fonts/LICENSE.txt",
    "assets/fonts/LICENSE-JetBrainsMono.txt",
)
IDENTITY_SOURCES = (
    "src/mcp/rpc.rs",
    "src/openrouter/api_types.rs",
    "src/tabby/mod.rs",
    "src/hf/fetch.rs",
)
PROJECT_LICENSE_MARKERS = {
    "LICENSE-MIT": ("MIT License", "Permission is hereby granted", "THE SOFTWARE IS PROVIDED"),
    "LICENSE-APACHE": (
        "Apache License",
        "Version 2.0",
        "TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION",
    ),
}
MACH_O_MAGICS = {
    b"\xfe\xed\xfa\xce",
    b"\xce\xfa\xed\xfe",
    b"\xfe\xed\xfa\xcf",
    b"\xcf\xfa\xed\xfe",
    b"\xca\xfe\xba\xbe",
    b"\xbe\xba\xfe\xca",
    b"\xca\xfe\xba\xbf",
    b"\xbf\xba\xfe\xca",
}
PE_MACHINES = {0x014C, 0x01C4, 0x8664, 0xAA64}
PE32_OPTIONAL_HEADER_SIZE = 224
PE32_PLUS_OPTIONAL_HEADER_SIZE = 240
HARDCODED_VERSION = re.compile(r"CodeWarp/\d|\"clientInfo\".*\"version\"\s*:\s*\"\d", re.DOTALL)


def package_metadata(root: Path) -> tuple[str, str]:
    """Read the package version and license expression from Cargo.toml."""
    cargo = (root / "Cargo.toml").read_bytes()
    package = tomllib.loads(cargo.decode("utf-8")).get("package")
    if not isinstance(package, dict):
        raise ValueError("Cargo.toml must define package version and license")
    version = package.get("version")
    license_expression = package.get("license")
    if not isinstance(version, str) or not isinstance(license_expression, str):
        raise ValueError("Cargo.toml must define package version and license")
    return version, license_expression


def is_valid_pe(contents: bytes) -> bool:
    """Validate bounded DOS, PE/COFF, optional-header, and section structures."""
    if len(contents) < 64 or not contents.startswith(b"MZ"):
        return False
    pe_offset = int.from_bytes(contents[60:64], "little")
    if pe_offset < 64 or pe_offset % 4 or pe_offset + 24 > len(contents):
        return False
    if contents[pe_offset : pe_offset + 4] != b"PE\0\0":
        return False

    coff_offset = pe_offset + 4
    machine = int.from_bytes(contents[coff_offset : coff_offset + 2], "little")
    section_count = int.from_bytes(contents[coff_offset + 2 : coff_offset + 4], "little")
    optional_size = int.from_bytes(contents[coff_offset + 16 : coff_offset + 18], "little")
    if machine not in PE_MACHINES or not 1 <= section_count <= 96:
        return False

    optional_offset = coff_offset + 20
    if optional_offset + optional_size > len(contents) or optional_size < 2:
        return False
    optional_magic = int.from_bytes(contents[optional_offset : optional_offset + 2], "little")
    minimum_optional_size = {
        0x010B: PE32_OPTIONAL_HEADER_SIZE,
        0x020B: PE32_PLUS_OPTIONAL_HEADER_SIZE,
    }.get(optional_magic)
    if minimum_optional_size is None or optional_size < minimum_optional_size:
        return False
    section_table_offset = optional_offset + optional_size
    return section_table_offset + section_count * 40 <= len(contents)


def is_native_executable(path: Path, contents: bytes) -> bool:
    """Recognize native Windows PE and Unix ELF/Mach-O executable files."""
    if contents.startswith(b"MZ"):
        return is_valid_pe(contents)
    try:
        mode = path.stat().st_mode
    except OSError:
        return False
    executable_bits = stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
    if not (mode & executable_bits):
        return False
    return contents.startswith(b"\x7fELF") or contents[:4] in MACH_O_MAGICS


def validate(root: Path, binary: Path | None) -> list[str]:
    """Return deterministic validation errors for the release inputs."""
    errors: list[str] = []
    try:
        version, license_expression = package_metadata(root)
    except (OSError, ValueError) as error:
        return [str(error)]

    if license_expression != "MIT OR Apache-2.0":
        errors.append(f"unexpected Cargo license expression: {license_expression}")

    for relative_path in REQUIRED_INPUTS:
        input_path = root / relative_path
        if not input_path.is_file():
            errors.append(f"missing archive input: {relative_path}")
        elif relative_path in PROJECT_LICENSE_MARKERS:
            try:
                license_text = input_path.read_text(encoding="utf-8")
            except (OSError, UnicodeError) as error:
                errors.append(f"cannot read project license {relative_path}: {error}")
            else:
                markers = PROJECT_LICENSE_MARKERS[relative_path]
                if not license_text.strip() or not all(marker in license_text for marker in markers):
                    errors.append(f"unrecognized or empty project license: {relative_path}")

    selected_binary = binary
    if selected_binary is None:
        candidates = (root / "target/release/codewarp.exe", root / "target/release/codewarp")
        selected_binary = next((candidate for candidate in candidates if candidate.is_file()), None)
    if selected_binary is None or not selected_binary.is_file():
        errors.append("missing release binary: pass --binary or build target/release/codewarp(.exe)")
    elif selected_binary.name not in {"codewarp", "codewarp.exe"}:
        errors.append(f"unexpected release binary name: {selected_binary.name}")
    else:
        try:
            binary_contents = selected_binary.read_bytes()
        except OSError as error:
            errors.append(f"cannot read release binary: {error}")
        else:
            if not is_native_executable(selected_binary, binary_contents):
                errors.append(f"release binary is not a native executable: {selected_binary}")
            elif b"CodeWarp" not in binary_contents or version.encode() not in binary_contents:
                errors.append(f"release binary does not contain Cargo version identity: {selected_binary}")

    for relative_path in IDENTITY_SOURCES:
        source_path = root / relative_path
        if not source_path.is_file():
            errors.append(f"missing identity source: {relative_path}")
            continue
        source = source_path.read_text(encoding="utf-8")
        if "env!(\"CARGO_PKG_VERSION\")" not in source:
            errors.append(f"identity source does not use Cargo metadata: {relative_path}")
        if HARDCODED_VERSION.search(source):
            errors.append(f"hard-coded CodeWarp version in identity source: {relative_path}")

    if errors:
        return errors
    return [f"validated CodeWarp {version} release inputs"]


def main() -> int:
    """Validate a repository selected by the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--binary", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    binary = args.binary.resolve() if args.binary is not None else None
    results = validate(root, binary)
    for result in results:
        print(result)
    success = len(results) == 1 and results[0].startswith("validated CodeWarp ")
    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
