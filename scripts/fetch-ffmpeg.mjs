// Downloads the LGPL FFmpeg build that Thira links against (issue #13) into
// third_party/ffmpeg/<package>/, verifying it against the checksums below.
//
// The packages come from the `ffmpeg-<package>` release, built by the
// "FFmpeg (LGPL)" workflow (.github/workflows/ffmpeg.yml). Each one holds the
// shared libraries, headers and pregenerated Rust bindings (binding.rs) for
// one platform. When changing the package, also update the paths in
// .cargo/config.toml.
//
// Usage: pnpm ffmpeg:fetch

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// FFmpeg version and package revision.
const PACKAGE = "8.1.3-2";
const RELEASE = `https://github.com/SreeshanthT/thira/releases/download/ffmpeg-${PACKAGE}`;
const SHA256 = {
  "linux-x64": "5d92581bee6cf4babe3bf065895613762f8a73d619fccba5719b95f80c78326e",
  "macos-arm64": "e3943ff241cb5edbe5a7ee0cbef32e1943a03c0fe4341341af7f06d38874f428",
  "macos-x64": "a4696123bf45d448a7e0a294ebc45e50f4d4a66b4ca12320a98feb51b9d7962c",
  "windows-x64": "2e4ea11153b996b74a67a8db535856ef733081081230f2f3b772ec5bfed1be79",
};

const os = { linux: "linux", darwin: "macos", win32: "windows" }[process.platform];
const arch = { x64: "x64", arm64: "arm64" }[process.arch];
const platform = `${os}-${arch}`;
if (!(platform in SHA256)) {
  console.error(`No FFmpeg package for ${process.platform}-${process.arch}.`);
  process.exit(1);
}

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "third_party", "ffmpeg");
const dest = join(root, PACKAGE);
const stamp = join(dest, ".sha256");
if (existsSync(stamp) && readFileSync(stamp, "utf8") === SHA256[platform]) {
  console.log(`FFmpeg ${PACKAGE} (${platform}) is already in ${dest}`);
  process.exit(0);
}

const name = `ffmpeg-${PACKAGE}-${platform}`;
console.log(`Downloading ${name}.tar.gz`);
const response = await fetch(`${RELEASE}/${name}.tar.gz`);
if (!response.ok) {
  console.error(`Download failed: HTTP ${response.status}`);
  process.exit(1);
}
const archive = Buffer.from(await response.arrayBuffer());
const actual = createHash("sha256").update(archive).digest("hex");
if (actual !== SHA256[platform]) {
  console.error(`Checksum mismatch for ${name}.tar.gz:\n  expected ${SHA256[platform]}\n  actual   ${actual}`);
  process.exit(1);
}

// Extract next to the destination, then swap it in, so an interrupted run
// never leaves a half-extracted tree behind a valid stamp.
const staging = join(root, `.staging-${process.pid}`);
rmSync(staging, { recursive: true, force: true });
mkdirSync(staging, { recursive: true });
const tarball = join(staging, `${name}.tar.gz`);
writeFileSync(tarball, archive);
// tar ships with Windows 10+, macOS and Linux. Relative paths keep Windows
// bsdtar from reading "C:" as a remote host.
execFileSync("tar", ["-xzf", `${name}.tar.gz`], { cwd: staging, stdio: "inherit" });
writeFileSync(join(staging, name, ".sha256"), SHA256[platform]);
rmSync(dest, { recursive: true, force: true });
renameSync(join(staging, name), dest);
rmSync(staging, { recursive: true, force: true });
console.log(`FFmpeg ${PACKAGE} (${platform}) is in ${dest}`);
