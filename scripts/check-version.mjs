#!/usr/bin/env node
// Checks that package.json, src-tauri/Cargo.toml and src-tauri/tauri.conf.json
// declare the same version, and that it matches the tag given as argument
// (e.g. `node scripts/check-version.mjs v0.1.0`).
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const versions = {
  "package.json": JSON.parse(read("package.json")).version,
  "src-tauri/tauri.conf.json": JSON.parse(read("src-tauri/tauri.conf.json")).version,
  "src-tauri/Cargo.toml": read("src-tauri/Cargo.toml").match(/^version\s*=\s*"([^"]+)"/m)?.[1],
};

const unique = new Set(Object.values(versions));
if (unique.size !== 1) {
  console.error("Version mismatch:", versions);
  process.exit(1);
}
const [version] = unique;
const tag = process.argv[2];
if (tag && tag !== `v${version}`) {
  console.error(`Tag ${tag} does not match version ${version} (expected v${version}).`);
  process.exit(1);
}
console.log(`Version ${version}${tag ? ` matches tag ${tag}` : ""}`);
