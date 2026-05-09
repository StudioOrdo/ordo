import { createWriteStream, readdirSync, statSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { join, relative, sep } from "node:path";

const root = process.cwd();
const outputPath = join(root, "project-export.txt");

const excludedDirectories = new Set([
  ".data",
  ".git",
  ".next",
  ".ordo",
  ".ordo-artifacts",
  ".runtime-logs",
  "build",
  "coverage",
  "dist",
  "node_modules",
  "out",
  "playwright-report",
  "target",
  "test-results",
]);

const excludedFiles = new Set([
  ".DS_Store",
  "project-export.txt",
  "tsconfig.tsbuildinfo",
]);

const excludedExtensions = new Set([
  ".db",
  ".ico",
  ".jpg",
  ".jpeg",
  ".key",
  ".lockb",
  ".log",
  ".pdf",
  ".png",
  ".sqlite",
  ".webp",
]);

function shouldSkip(path) {
  const relativePath = relative(root, path);
  const parts = relativePath.split(sep);
  const name = parts.at(-1);

  if (!name) {
    return false;
  }

  if (parts.some((part) => excludedDirectories.has(part))) {
    return true;
  }

  if (parts[0] === "docs" && parts.some((part) => part.startsWith("_"))) {
    return true;
  }

  if (excludedFiles.has(name)) {
    return true;
  }

  if (name.startsWith(".env")) {
    return true;
  }

  const extension = name.includes(".") ? name.slice(name.lastIndexOf(".")).toLowerCase() : "";
  return excludedExtensions.has(extension);
}

function collectFiles(directory) {
  const entries = readdirSync(directory, { withFileTypes: true })
    .sort((left, right) => left.name.localeCompare(right.name));
  const files = [];

  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (shouldSkip(path)) {
      continue;
    }

    if (entry.isDirectory()) {
      files.push(...collectFiles(path));
    } else if (entry.isFile()) {
      files.push(path);
    }
  }

  return files;
}

function isLikelyText(buffer) {
  if (buffer.includes(0)) {
    return false;
  }
  return true;
}

function byteCount(path) {
  return statSync(path).size;
}

const files = collectFiles(root);
const stream = createWriteStream(outputPath, { encoding: "utf8" });
let exportedFiles = 0;
let skippedBinaryFiles = 0;

stream.write(`# Studio Ordo Project Export\n`);
stream.write(`Generated: ${new Date().toISOString()}\n`);
stream.write(`Root: ${root}\n\n`);
stream.write(`Excluded directories: ${[...excludedDirectories].sort().join(", ")}\n`);
stream.write(`Excluded private docs: docs/_*/\n`);
stream.write(`Excluded file patterns: .env*, ${[...excludedFiles].sort().join(", ")}\n`);
stream.write(`Excluded extensions: ${[...excludedExtensions].sort().join(", ")}\n\n`);

for (const path of files) {
  const relativePath = relative(root, path);
  const buffer = await readFile(path);

  if (!isLikelyText(buffer)) {
    skippedBinaryFiles += 1;
    continue;
  }

  exportedFiles += 1;
  stream.write(`\n\n===== FILE: ${relativePath} =====\n\n`);
  stream.write(buffer.toString("utf8"));
  if (!buffer.toString("utf8").endsWith("\n")) {
    stream.write("\n");
  }
}

stream.write(`\n\n===== EXPORT SUMMARY =====\n\n`);
stream.write(`Exported files: ${exportedFiles}\n`);
stream.write(`Skipped binary files: ${skippedBinaryFiles}\n`);
stream.end();

stream.on("finish", () => {
  const size = byteCount(outputPath);
  console.log(`Exported ${exportedFiles} files to project-export.txt (${size} bytes).`);
  if (skippedBinaryFiles > 0) {
    console.log(`Skipped ${skippedBinaryFiles} binary-looking files.`);
  }
});
