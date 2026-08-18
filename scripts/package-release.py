#!/usr/bin/env python3
"""Build a validated, portable CodeWarp release archive."""

from __future__ import annotations

import argparse
import importlib.util
import sys
import zipfile
from pathlib import Path


def load_validator():
    validator_path = Path(__file__).with_name("validate-release-manifest.py")
    spec = importlib.util.spec_from_file_location("codewarp_release_validator", validator_path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load release validator: {validator_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


_VALIDATOR = load_validator()
REQUIRED_INPUTS = _VALIDATOR.REQUIRED_INPUTS
package_metadata = _VALIDATOR.package_metadata
validate = _VALIDATOR.validate


def resolve_binary(root: Path, requested: Path | None) -> Path:
    if requested is not None:
        return requested.resolve()
    candidates = (root / "target/release/codewarp.exe", root / "target/release/codewarp")
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError("missing release binary: pass --binary or build target/release/codewarp(.exe)")


def archive_name(version: str, platform: str) -> str:
    normalized = platform.strip()
    if not normalized or any(char in normalized for char in '/\\'):
        raise ValueError("platform must be a non-empty archive-safe label")
    return f"codewarp-{version}-{normalized}.zip"


def write_archive(root: Path, binary: Path, output: Path, version: str) -> None:
    package_root = f"codewarp-{version}"
    binary_name = "codewarp.exe" if binary.suffix.lower() == ".exe" else "codewarp"
    source_entries = [(binary_name, binary)] + [
        (relative_path, root / relative_path) for relative_path in REQUIRED_INPUTS
    ]

    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        output.unlink()

    with zipfile.ZipFile(
        output,
        mode="w",
        compression=zipfile.ZIP_DEFLATED,
        compresslevel=9,
    ) as archive:
        for relative_path, source in source_entries:
            info = zipfile.ZipInfo(f"{package_root}/{relative_path}")
            info.date_time = (1980, 1, 1, 0, 0, 0)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (0o755 if relative_path == binary_name else 0o644) << 16
            archive.writestr(info, source.read_bytes())

    expected = {f"{package_root}/{relative_path}" for relative_path, _ in source_entries}
    with zipfile.ZipFile(output) as archive:
        actual = set(archive.namelist())
    if actual != expected:
        raise RuntimeError(f"archive contents mismatch: expected {sorted(expected)}, got {sorted(actual)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output-dir", type=Path, default=Path("dist"))
    parser.add_argument("--platform", default="")
    args = parser.parse_args()

    root = args.root.resolve()
    try:
        version, _ = package_metadata(root)
        binary = resolve_binary(root, args.binary)
        errors = validate(root, binary)
        if len(errors) != 1 or not errors[0].startswith("validated CodeWarp "):
            for error in errors:
                print(error, file=sys.stderr)
            return 1

        platform = args.platform or ("windows-x86_64" if binary.suffix.lower() == ".exe" else "linux-x86_64")
        output = args.output_dir.resolve() / archive_name(version, platform)
        write_archive(root, binary, output, version)
    except (FileNotFoundError, OSError, ValueError, RuntimeError) as error:
        print(error, file=sys.stderr)
        return 1

    print(f"created {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
