#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0

"""Verify native rebuild boundaries using real compiler invocations.

Run from a checkout with Python 3, Cargo/rustfmt, CMake, and a C++20 compiler:

    python3 tools/check_native_rebuilds.py --jobs 4

This is intentionally a slower build regression check, not part of cargo test.
It creates a disposable copy and a tiny consumer with a path dependency on that
copy, preserving the checkout and its target directory. The checkout's Cargo.lock
seeds the fixture; Cargo only adds the local consumer workspace package. Native
dependencies are downloaded and built by CMake in the disposable target directory.

The first pass warms check/build/test; Cargo can assign these different build
variants and perform more than one initial native build. Subsequent phases require
zero upstream Slang/fmt compiler invocations after ordinary Rust/test/bridge
edits, and at least one after changing CXXFLAGS. This last phase normally rebuilds
all Slang sources.
--skip-config-change skips that expensive positive control for local iteration;
omit it for the full check. --keep retains commands, compiler JSONL
logs, a JSON summary, and Cargo output. Failed runs are always retained.

No timings are asserted: the test counts source compilations, not build-script
executions, CMake configuration, compiler discovery probes, or linker calls.
"""

import argparse
import collections
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import time


LAUNCHER = r'''#!PYTHON
# SPDX-License-Identifier: Apache-2.0
import json
import os
from pathlib import Path
import sys

args = sys.argv[1:]
if "-c" in args:
    sources = [str(Path(arg).resolve()) for arg in args
               if not arg.startswith("-")
               and Path(arg).suffix in {".c", ".cc", ".cpp", ".cxx", ".C"}]
    record = json.dumps({"sources": sources, "args": args}) + "\n"
    descriptor = os.open(os.environ["SLANG_RS_REBUILD_LOG"],
                         os.O_APPEND | os.O_CREAT | os.O_WRONLY, 0o600)
    try:
        os.write(descriptor, record.encode())
    finally:
        os.close(descriptor)
compiler = json.loads(os.environ["SLANG_RS_REBUILD_REAL_CXX"])
os.execvp(compiler[0], compiler + args)
'''

CONSUMER_MANIFEST = '''[package]
name = "slang-rebuild-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
slang-rs = { path = ".." }
'''

CONSUMER_SOURCE = '''// SPDX-License-Identifier: Apache-2.0
pub fn parse(source: &str) -> usize {
    let source = slang_rs::Source::Text { name: "top.sv", text: source };
    let config = slang_rs::SlangConfig {
        sources: &[source],
        ..Default::default()
    };
    slang_rs::Compilation::new(&config).unwrap().ports().unwrap()["top"].len()
}
'''

CONSUMER_TEST = '''// SPDX-License-Identifier: Apache-2.0
#[test]
fn downstream_native_import() {
    assert_eq!(slang_rebuild_consumer::parse("module top(input a); endmodule"), 1);
}
'''


def classify(record, upstream_sources=()):
    """Classify real source files; deliberately ignore CMake compiler probes."""
    for source in record["sources"]:
        source = source.replace("\\", "/")
        if "/CMakeFiles/" in source or "/CMakeScratch/" in source or Path(source).name in {
            "CMakeCXXCompilerId.cpp", "testCXXCompiler.cxx", "CheckCXXSourceCompiles.cxx"
        }:
            continue
        if "/cpp/native.cpp" in source or "/cxxbridge/" in source:
            return "bridge"
        # The bridge and upstream libraries now share the package's OUT_DIR.
        # Only CMake's native subdirectory contains upstream build inputs.
        if "/out/native/" in source or any(
            source == prefix or source.startswith(prefix + "/")
            for prefix in upstream_sources
        ):
            return "slang"
    return "other"


class Check:
    def __init__(self, root, copy, environment, jobs):
        self.root = root
        self.copy = copy
        self.environment = environment
        self.jobs = jobs
        self.results = []

    def run(self, name, arguments, *, slang=None, bridge=None):
        log = self.root / (name + ".compiler.jsonl")
        output = self.root / (name + ".cargo.log")
        environment = dict(self.environment, SLANG_RS_REBUILD_LOG=str(log))
        upstream_sources = tuple(
            str(Path(environment[key]).resolve()).replace("\\", "/").rstrip("/")
            for key in ("FETCHCONTENT_SOURCE_DIR_SLANG", "FETCHCONTENT_SOURCE_DIR_FMT")
            if environment.get(key)
        )
        command = ["cargo", *arguments]
        if arguments[0] in {"check", "build", "test"}:
            command.extend(["--locked", "--jobs", str(self.jobs)])
        print(f"{name}: {shlex.join(command)}", flush=True)
        started = time.monotonic()
        with output.open("w") as stream:
            status = subprocess.run(
                command, cwd=self.copy, env=environment,
                stdout=stream, stderr=subprocess.STDOUT, check=False,
            ).returncode
        records = [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []
        counts = collections.Counter(classify(record, upstream_sources) for record in records)
        result = {
            "phase": name, "command": command, "exit_code": status,
            "seconds": round(time.monotonic() - started, 2),
            "slang": counts["slang"], "bridge": counts["bridge"], "other": counts["other"],
        }
        self.results.append(result)
        (self.root / "report.json").write_text(json.dumps(self.results, indent=2) + "\n")
        print(
            f"  Slang={counts['slang']} bridge={counts['bridge']} "
            f"other/probes={counts['other']} ({result['seconds']}s)", flush=True,
        )
        if status:
            print("\n".join(output.read_text(errors="replace").splitlines()[-60:]), file=sys.stderr)
            raise RuntimeError(f"{name}: Cargo failed; full output: {output}")
        for category, expected in (("slang", slang), ("bridge", bridge)):
            actual = counts[category]
            if expected == "none" and actual != 0:
                raise RuntimeError(f"{name}: expected no {category} compilation, found {actual}; see {log}")
            if expected == "some" and actual == 0:
                raise RuntimeError(f"{name}: expected {category} compilation; instrumentation or invalidation failed")
        return counts


def add_comment(path):
    with path.open("a") as stream:
        stream.write("\n// Native rebuild regression: irrelevant comment edit.\n")


def run_checks(args, root):
    copy = root / "slang-rs"
    shutil.copytree(
        args.source, copy,
        ignore=shutil.ignore_patterns(".git", "target", ".venv", "__pycache__"),
    )
    manifest = copy / "Cargo.toml"
    with manifest.open("a") as stream:
        stream.write('\n[workspace]\nmembers = [".", "rebuild-consumer"]\nresolver = "3"\n')
    consumer = copy / "rebuild-consumer"
    (consumer / "src").mkdir(parents=True)
    (consumer / "tests").mkdir()
    (consumer / "Cargo.toml").write_text(CONSUMER_MANIFEST)
    (consumer / "src/lib.rs").write_text(CONSUMER_SOURCE)
    (consumer / "tests/import.rs").write_text(CONSUMER_TEST)

    # Existing registry pins stay unchanged; this package introduces no new
    # registry dependencies. Add its exact local lock entry to avoid unlocking
    # or re-resolving the dependency graph merely to create the consumer.
    with (copy / "Cargo.lock").open("a") as stream:
        stream.write('\n[[package]]\nname = "slang-rebuild-consumer"\nversion = "0.0.0"\n'
                     'dependencies = ["slang-rs"]\n')

    compiler = shlex.split(args.cxx)
    if not compiler or shutil.which(compiler[0]) is None:
        raise RuntimeError(f"C++ compiler not found: {args.cxx}")
    compiler[0] = shutil.which(compiler[0])
    launcher = root / "record-cxx"
    launcher.write_text(LAUNCHER.replace("#!PYTHON", f"#!{sys.executable}", 1))
    launcher.chmod(0o755)
    environment = dict(os.environ)
    environment.update({
        "CXX": str(launcher),
        "SLANG_RS_REBUILD_REAL_CXX": json.dumps(compiler),
        "CARGO_TARGET_DIR": str(root / "target"),
        "CARGO_TERM_COLOR": "never",
        # A compiler cache hit must not hide an unnecessary rebuild. Slang's
        # upstream CMake also discovers ccache automatically on some machines.
        "CCACHE_DISABLE": "1",
    })
    # Make a previously inherited global target directory irrelevant, but keep
    # legitimate Rust/C++ compiler flags. The same flags apply to every phase.
    check = Check(root, copy, environment, args.jobs)
    check.run("warm-check", ["check", "--workspace"], slang="some", bridge="some")
    # Cargo may choose different build-script dependency artifacts for check,
    # build and test. Warm each variant once before asserting repeated reuse.
    check.run("warm-build", ["build", "--workspace"])
    finish_checks(check, args)


def finish_checks(check, args):
    copy = check.copy
    consumer = copy / "rebuild-consumer"
    # Run the consumer test to measure downstream build reuse; repository API
    # coverage is exercised by the normal test suite independently.
    check.run("warm-test", ["test", "-p", "slang-rebuild-consumer"])

    check.run("repeat-check", ["check", "--workspace"], slang="none", bridge="none")
    check.run("repeat-build", ["build", "--workspace"], slang="none", bridge="none")
    check.run("repeat-test", ["test", "-p", "slang-rebuild-consumer"], slang="none", bridge="none")

    add_comment(copy / "src/lib.rs")
    check.run("rust-library-edit", ["check", "--workspace"], slang="none", bridge="none")
    add_comment(consumer / "src/lib.rs")
    check.run("rust-consumer-edit", ["build", "--workspace"], slang="none", bridge="none")
    add_comment(consumer / "tests/import.rs")
    check.run("rust-test-edit", ["test", "-p", "slang-rebuild-consumer"], slang="none", bridge="none")

    # Cargo tracks the file containing the CXX bridge; regeneration stays in the
    # bridge, even when the source change does not affect generated C++.
    add_comment(copy / "src/native.rs")
    check.run("rust-native-edit", ["build", "--workspace"], slang="none", bridge="some")
    check.run("after-rust-native-edit", ["check", "--workspace"], slang="none")

    add_comment(copy / "cpp/native.cpp")
    check.run("bridge-edit", ["build", "--workspace"], slang="none", bridge="some")
    # If check/build use different bridge variants, each needs to pick up this
    # bridge edit once. Neither may recompile upstream Slang.
    check.run("after-bridge-edit", ["check", "--workspace"], slang="none")
    finish_after_bridge(check, args)


def finish_after_bridge(check, args):
    environment = check.environment
    check.run("repeat-after-bridge-check", ["check", "--workspace"], slang="none", bridge="none")
    check.run("repeat-after-bridge-build", ["build", "--workspace"], slang="none", bridge="none")

    # Format the disposable fixture first, since the hand-written consumer may
    # differ from the installed rustfmt. Both formatting modes must be inert to
    # native compilation, even with modified Rust files.
    check.run("format", ["fmt", "--all"], slang="none", bridge="none")
    check.run("format-check", ["fmt", "--all", "--", "--check"], slang="none", bridge="none")
    # Formatting can update src/native.rs, which legitimately recompiles the
    # bridge on the next Cargo invocation. It must never rebuild Slang itself.
    check.run("after-format", ["check", "--workspace"], slang="none")

    if args.skip_config_change:
        print("native-config-change: SKIPPED (partial local check)", flush=True)
    else:
        environment["CXXFLAGS"] = (
            environment.get("CXXFLAGS", "") + " -DSLANG_RS_REBUILD_CHECK=1"
        ).strip()
        check.run("native-config-change", ["build", "--workspace"], slang="some")
        check.run("repeat-native-config", ["build", "--workspace"], slang="none", bridge="none")
    print(f"PASS: {len(check.results)} phases; summary: {check.root / 'report.json'}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--source", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--jobs", type=int, default=min(4, os.cpu_count() or 1))
    parser.add_argument("--cxx", default=os.environ.get("CXX", "c++"), help="C++20 compiler command")
    parser.add_argument("--keep", action="store_true", help="retain the disposable fixture after success")
    parser.add_argument("--skip-config-change", action="store_true", help="skip the second full native build (local use)")
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    root = Path(tempfile.mkdtemp(prefix="slang-native-rebuild-"))
    print(f"Disposable build fixture: {root}", flush=True)
    try:
        run_checks(args, root)
    except (OSError, RuntimeError) as error:
        print(f"FAIL: {error}\nRetained fixture and logs: {root}", file=sys.stderr)
        return 1
    if args.keep:
        print(f"Retained fixture and logs: {root}")
    else:
        shutil.rmtree(root)
    return 0


if __name__ == "__main__":
    sys.exit(main())
