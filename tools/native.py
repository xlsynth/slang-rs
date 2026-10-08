#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0

"""Build, install, or archive slang-rs's pinned native dependencies.

This deliberately uses the same CMake project as Cargo. It installs native
headers and static libraries only; the Rust crate and its bridge stay local.
"""

import argparse
import gzip
import hashlib
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile


REPO = Path(__file__).resolve().parent.parent
MANIFEST = Path("share/manifest.txt")
LIBRARIES = (Path("lib/libsvlang.a"), Path("lib/libfmt.a"))
TREES = (Path("include"), Path("share/licenses"))


def manifest(prefix):
    try:
        values = dict(line.split("=", 1) for line in (prefix / MANIFEST).read_text().splitlines())
        # These fields form the default installation path. Cargo verifies
        # the actual native recipe and target when it consumes this prefix.
        for key in ("native_recipe_sha256", "slang_version", "system", "processor"):
            value = values[key]
            if not value or Path(value).name != value:
                raise ValueError(f"invalid {key}")
        return values
    except (OSError, KeyError, ValueError) as exc:
        raise ValueError(f"{prefix} is not a native installation: {exc}") from exc


def payload(prefix):
    """Yield only portable native paths; never build files, pkg-config, or Rust."""
    manifest(prefix)
    for path in (*LIBRARIES, MANIFEST):
        source = prefix / path
        if not source.is_file() or source.is_symlink():
            raise ValueError(f"missing regular native file: {source}")
    for tree in TREES:
        source = prefix / tree
        if not source.is_dir() or source.is_symlink():
            raise ValueError(f"missing native directory: {source}")

    yield Path("lib")
    yield from LIBRARIES
    yield Path("share")
    yield MANIFEST
    for tree in TREES:
        yield tree
        for directory, dirs, files in os.walk(prefix / tree):
            dirs.sort()
            files.sort()
            for name in dirs + files:
                source = Path(directory) / name
                if source.is_symlink():
                    raise ValueError(f"unexpected symlink in native installation: {source}")
                yield source.relative_to(prefix)


def copy_payload(source, destination, *, owner=None, group=None, file_mode=None, dir_mode=None):
    paths = list(payload(source))  # Validate everything before copying, especially for sudo.
    if source.resolve() == destination.resolve():
        raise ValueError("source and destination must be different directories")
    if destination.is_symlink():
        raise ValueError(f"destination may not be a symlink: {destination}")
    destination.mkdir(parents=True, exist_ok=True)
    directories = [destination]
    files = []
    for relative in paths:
        src = source / relative
        dst = destination / relative
        if dst.is_symlink():
            raise ValueError(f"destination may not contain symlinks: {dst}")
        if src.is_dir():
            dst.mkdir(exist_ok=True)
            directories.append(dst)
        else:
            # Unlike copy2, copyfile gives new files the caller's umask / the
            # destination's default ACL and group. Don't import staging modes.
            shutil.copyfile(src, dst)
            files.append(dst)
    # Let the system validate owner, group, and modes. No -R: change only
    # paths in this payload. Set parent directories last for read-only modes.
    if owner is not None or group is not None:
        spec = owner or ""
        if group is not None:
            spec += ":" + group
        subprocess.run(["chown", "--", spec, *files, *reversed(directories)], check=True)
    if file_mode is not None:
        subprocess.run(["chmod", "--", file_mode, *files], check=True)
    if dir_mode is not None:
        subprocess.run(["chmod", "--", dir_mode, *reversed(directories)], check=True)


def build(build_dir, stage, cmake_args):
    build_dir = build_dir.resolve()
    # Pass through the same source overrides as build.rs; clearing an unset
    # override prevents an earlier CMake cache entry from being reused.
    options = [
        f"-D{key}={os.environ.get(key, '')}"
        for key in ("FETCHCONTENT_SOURCE_DIR_SLANG", "FETCHCONTENT_SOURCE_DIR_FMT")
    ]
    if (
        sys.platform == "darwin"
        and not os.environ.get("CMAKE_TOOLCHAIN_FILE")
        and not any(
            arg.startswith(("-DCMAKE_OSX_ARCHITECTURES", "-DCMAKE_TOOLCHAIN_FILE"))
            for arg in cmake_args
        )
    ):
        # Python or CMake can run under Rosetta on Apple Silicon. Choose the
        # machine's native architecture by default, as Cargo/rustup normally
        # does, rather than accidentally installing an Intel-only library.
        arm64 = subprocess.run(
            ["sysctl", "-n", "hw.optional.arm64"], capture_output=True, text=True, check=False
        )
        if arm64.stdout.strip() == "1":
            options.append("-DCMAKE_OSX_ARCHITECTURES=arm64")
    subprocess.run(
        [
            "cmake", "-S", str(REPO / "cmake"), "-B", str(build_dir),
            f"-DPython_EXECUTABLE={sys.executable}",
            *options, *cmake_args,
        ],
        check=True,
    )
    subprocess.run(
        [
            "cmake", "--build", str(build_dir), "--config", "Release", "--parallel",
            os.environ.get("CMAKE_BUILD_PARALLEL_LEVEL", str(os.cpu_count() or 1)),
        ],
        check=True,
    )
    subprocess.run(
        ["cmake", "--install", str(build_dir), "--config", "Release", "--prefix", str(stage)],
        check=True,
    )


def archive(source, output):
    paths = list(payload(source))
    output.parent.mkdir(parents=True, exist_ok=True)
    # Avoid paths or timestamps leaking from the staging tree into CI archives.
    # Archive entries are prefix-relative; extract them directly to a prefix.
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "0"))
    with (
        output.open("wb") as raw,
        gzip.GzipFile(filename="", fileobj=raw, mode="wb", mtime=epoch) as compressed,
        tarfile.open(fileobj=compressed, mode="w") as tar,
    ):
        for path in paths:
            src = source / path
            entry = tarfile.TarInfo(path.as_posix())
            entry.mtime = epoch
            if src.is_dir():
                entry.type = tarfile.DIRTYPE
                entry.mode = 0o755
                tar.addfile(entry)
            else:
                entry.size = src.stat().st_size
                entry.mode = 0o644
                with src.open("rb") as file:
                    tar.addfile(entry, file)
    digest = hashlib.sha256()
    with output.open("rb") as file:
        for block in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(block)
    checksum = Path(f"{output}.sha256")
    checksum.write_text(f"{digest.hexdigest()}  {output.name}\n")
    print(f"Archive:  {output}")
    print(f"SHA-256:  {checksum}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build_parser = commands.add_parser("build", help="build a portable native staging directory")
    install_parser = commands.add_parser("install", help="install for this user or at --prefix")
    archive_parser = commands.add_parser("archive", help="create a CI tar.gz archive and SHA-256")
    for command in (build_parser, install_parser, archive_parser):
        command.add_argument(
            "--build-dir", type=Path, default=REPO / "target/native",
            help="CMake build tree (default: %(default)s)",
        )
        command.add_argument(
            "--cmake-arg", action="append", default=[],
            help="extra CMake configure argument; repeat as needed, e.g. "
            "--cmake-arg=-DCMAKE_TOOLCHAIN_FILE=/path/to/toolchain.cmake",
        )
    for command in (build_parser, archive_parser):
        command.add_argument("--output", type=Path, required=True, help="destination stage / tar.gz")
    for command in (install_parser, archive_parser):
        command.add_argument(
            "--from", dest="source", type=Path,
            help="use an existing native staging directory; skip the build",
        )
    install_parser.add_argument(
        "--prefix", type=Path,
        help="default: ~/.local/lib/slang-rs/native/slang-<version>/<recipe>-<system>-<processor>",
    )
    install_parser.add_argument("--owner", help="user name or numeric UID, as accepted by chown")
    install_parser.add_argument("--group", help="group name or numeric GID, as accepted by chown")
    install_parser.add_argument(
        "--file-mode", help="mode as accepted by chmod, e.g. 0644; defaults to umask"
    )
    install_parser.add_argument(
        "--dir-mode", help="mode as accepted by chmod, e.g. 2755; defaults to parent and umask"
    )
    args = parser.parse_args()
    try:
        with tempfile.TemporaryDirectory(prefix="slang-rs-native-") as temporary:
            source = getattr(args, "source", None)
            if source is None:
                source = Path(temporary)
                build(args.build_dir, source, args.cmake_arg)
            source = source.expanduser().resolve()
            if args.command == "build":
                destination = args.output.expanduser().absolute()
                copy_payload(source, destination)
                print(f"Staged native libraries: {destination}")
            elif args.command == "archive":
                archive(source, args.output.expanduser().absolute())
            else:
                values = manifest(source)
                platform = f"{values['system'].lower()}-{values['processor']}"
                default = (
                    Path.home() / ".local/lib/slang-rs/native"
                    / f"slang-{values['slang_version']}"
                    / f"{values['native_recipe_sha256'][:16]}-{platform}"
                )
                destination = (args.prefix or default).expanduser().absolute()
                copy_payload(
                    source, destination, owner=args.owner, group=args.group,
                    file_mode=args.file_mode, dir_mode=args.dir_mode,
                )
                print(f"Installed Slang {values['slang_version']} native libraries: {destination}")
                print(f"export SLANG_RS_NATIVE_DIR={shlex.quote(str(destination))}")
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        parser.exit(1, f"{parser.prog}: {exc}\n")


if __name__ == "__main__":
    main()
