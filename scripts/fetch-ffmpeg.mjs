// Downloads the LGPL FFmpeg build that Thira links against (issue #13) into
// third_party/ffmpeg/<version>/, verifying it against the checksums below.
//
// The packages come from the `ffmpeg-<version>` release, built by the
// "FFmpeg (LGPL)" workflow (.github/workflows/ffmpeg.yml). When bumping the
// version, also update the paths in .cargo/config.toml.
//
// Usage: pnpm ffmpeg:fetch

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const VERSION = "8.1.3";
const RELEASE = `https://github.com/SreeshanthT/thira/releases/download/ffmpeg-${VERSION}`;
const SHA256 = {
  "linux-x64": "1e3ccb2d81b1306b0d28ecf219c242b6ae9d86a1815de4d92025a4da8aff1b89",
  "macos-arm64": "211384474df3d0ef3c177ded4b836a79263958d9baffbfd872f4ecf64ba0f4a1",
  "macos-x64": "ad6e46666811e9e7a768bf043df2d6c12a59ac87b8e3c7a4440c2bfdffb3c3d5",
  "windows-x64": "96b54d1e73b1a56a4297b96087baa6fdbac06f6b7680099e99e8a9d6356b51af",
};

const os = { linux: "linux", darwin: "macos", win32: "windows" }[process.platform];
const arch = { x64: "x64", arm64: "arm64" }[process.arch];
const platform = `${os}-${arch}`;
if (!(platform in SHA256)) {
  console.error(`No FFmpeg package for ${process.platform}-${process.arch}.`);
  process.exit(1);
}

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "third_party", "ffmpeg");
const dest = join(root, VERSION);
const stamp = join(dest, ".sha256");
if (existsSync(stamp) && readFileSync(stamp, "utf8") === SHA256[platform]) {
  console.log(`FFmpeg ${VERSION} (${platform}) is already in ${dest}`);
  process.exit(0);
}

const name = `ffmpeg-${VERSION}-${platform}`;
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
console.log(`FFmpeg ${VERSION} (${platform}) is in ${dest}`);
