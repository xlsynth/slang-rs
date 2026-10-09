#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0

"""Local checks for installation permissions and portable archive contents."""

import hashlib
import os
from pathlib import Path
import stat
import tarfile
import tempfile
import unittest

import native


class NativeInstallTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source = self.root / "stage"
        for path, content in {
            native.MANIFEST: (
                "native_recipe_sha256=" + "a" * 64 + "\n"
                "slang_version=11.0\nfmt_version=12.1.0\nsystem=Linux\nprocessor=x86_64\n"
                "cxx_compiler_id=GNU\ncxx_compiler_version=13.3.0\n"
            ),
            Path("include/slang/slang_export.h"): "// test header\n",
            Path("include/fmt/format.h"): "// test fmt header\n",
            Path("share/licenses/slang-LICENSE"): "license\n",
            Path("lib/libsvlang.a"): "test Slang archive\n",
            Path("lib/libfmt.a"): "test fmt archive\n",
            Path("lib/pkgconfig/slang.pc"): "bad absolute build path",
            Path("lib/cmake/Slang/SlangTargets.cmake"): "bad absolute build path",
            Path("build/libslang-rs-bridge.a"): "bridge must stay local",
            Path("share/doc/slang/build-notes.txt"): "not part of the bundle",
        }.items():
            (self.source / path).parent.mkdir(parents=True, exist_ok=True)
            (self.source / path).write_text(content)

    def test_defaults_follow_umask_and_overrides_touch_only_payload(self):
        prefix = self.root / "shared/native"
        old_umask = os.umask(0o027)
        try:
            native.copy_payload(self.source, prefix)
        finally:
            os.umask(old_umask)
        mode = lambda path: stat.S_IMODE(path.stat().st_mode)
        self.assertEqual(mode(prefix / "include"), 0o750)
        self.assertEqual(mode(prefix / "include/fmt/format.h"), 0o640)
        unrelated = prefix / "unrelated"
        unrelated.write_text("existing file outside the payload")
        unrelated.chmod(0o604)
        parent_mode = mode(prefix.parent)
        native.copy_payload(
            self.source, prefix, owner=str(os.getuid()), group=str(os.getgid()),
            file_mode="0444", dir_mode="0555",
        )
        self.assertEqual(mode(prefix / "lib/libsvlang.a"), 0o444)
        self.assertEqual(mode(prefix / "share/licenses"), 0o555)
        self.assertEqual(mode(unrelated), 0o604)
        self.assertEqual(mode(prefix.parent), parent_mode)
        # Permit TemporaryDirectory cleanup after testing a read-only install.
        for directory, _, _ in os.walk(prefix):
            Path(directory).chmod(0o755)

    def test_archive_contains_only_native_files_and_is_deterministic(self):
        archive = self.root / "native.tar.gz"
        native.archive(self.source, archive)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        self.assertEqual(
            Path(f"{archive}.sha256").read_text(),
            f"{digest}  native.tar.gz\n",
        )
        with tarfile.open(archive) as tar:
            paths = tar.getnames()
            self.assertIn("include/slang/slang_export.h", paths)
            self.assertIn("lib/libsvlang.a", paths)
            self.assertIn("lib/libfmt.a", paths)
            self.assertIn(str(native.MANIFEST), paths)
            self.assertIn("share/licenses/slang-LICENSE", paths)
            self.assertFalse(
                any("pkgconfig" in p or "cmake" in p or "bridge" in p or "share/doc" in p
                    for p in paths)
            )
            for member in tar:
                self.assertEqual((member.uid, member.gid, member.uname, member.gname), (0, 0, "", ""))
                self.assertFalse(member.name.startswith("/"))
        os.utime(self.source / "lib/libsvlang.a", (100, 100))
        self.source.joinpath("lib/libsvlang.a").chmod(0o600)
        native.archive(self.source, archive)
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(), digest)

    def test_reject_symlink_in_shared_destination(self):
        prefix = self.root / "shared/native"
        (prefix / "include").mkdir(parents=True)
        unrelated = self.root / "other-files"
        unrelated.mkdir()
        (unrelated / "format.h").write_text("do not overwrite")
        (prefix / "include/fmt").symlink_to(unrelated)
        with self.assertRaisesRegex(ValueError, "symlink"):
            native.copy_payload(self.source, prefix, file_mode="0444")
        self.assertEqual((unrelated / "format.h").read_text(), "do not overwrite")

    def test_reject_missing_library_before_install(self):
        (self.source / "lib/libfmt.a").unlink()
        prefix = self.root / "new-prefix"
        with self.assertRaisesRegex(ValueError, "native file"):
            native.copy_payload(self.source, prefix)
        self.assertFalse(prefix.exists())


if __name__ == "__main__":
    unittest.main()
