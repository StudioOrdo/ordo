import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

export async function loadLocalEnv(repoRoot, baseEnv = process.env) {
  const env = { ...baseEnv };
  const envPath = resolve(repoRoot, ".env.local");

  let raw;
  try {
    raw = await readFile(envPath, "utf8");
  } catch (error) {
    if (error?.code === "ENOENT") {
      return env;
    }
    throw error;
  }

  for (const line of raw.split(/\r?\n/)) {
    const parsed = parseEnvLine(line);
    if (!parsed) {
      continue;
    }
    const [key, value] = parsed;
    if (env[key] === undefined) {
      env[key] = value;
    }
  }

  return env;
}

function parseEnvLine(line) {
  const trimmed = line.trim();
  if (!trimmed || trimmed.startsWith("#")) {
    return null;
  }

  const equalsIndex = trimmed.indexOf("=");
  if (equalsIndex <= 0) {
    return null;
  }

  const key = trimmed.slice(0, equalsIndex).trim();
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(key)) {
    return null;
  }

  const rawValue = trimmed.slice(equalsIndex + 1).trim();
  return [key, unquoteValue(rawValue)];
}

function unquoteValue(value) {
  if (value.length >= 2 && value.startsWith('"') && value.endsWith('"')) {
    return value.slice(1, -1).replaceAll('\\n', '\n').replaceAll('\\"', '"').replaceAll('\\\\', '\\');
  }
  if (value.length >= 2 && value.startsWith("'") && value.endsWith("'")) {
    return value.slice(1, -1);
  }
  return value;
}