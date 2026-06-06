"use strict";

const fs = require("fs/promises");
const path = require("path");
const os = require("os");
const { execFile } = require("child_process");
const { promisify } = require("util");

const execFileAsync = promisify(execFile);
const MAX_BUFFER = 10 * 1024 * 1024;

async function formatSnapshot(options) {
  return withSnapshot(options, async (snapshotPath) => {
    const { stdout } = await runNodia(
      options.executable,
      ["fmt", "--stdout", snapshotPath],
      options.cwd,
    );
    return stdout;
  });
}

async function checkSnapshot(options) {
  return withSnapshot(options, async (snapshotPath) => {
    const result = await runCheck(snapshotPath, options.executable, options.cwd);
    return normalizeCheckResult(result, snapshotPath, options.filePath);
  });
}

async function runCheck(filePath, executable, cwd) {
  try {
    const { stdout, stderr } = await runNodia(
      executable,
      ["check", filePath, "--json"],
      cwd,
    );
    return parseCheckPayload(stdout || stderr || '{"ok":true,"errors":[]}');
  } catch (error) {
    const payload = parseCheckPayload(error.stdout || error.stderr || "");
    if (payload) {
      return payload;
    }
    throw error;
  }
}

async function runNodia(executable, args, cwd) {
  return execFileAsync(executable, args, {
    cwd,
    encoding: "utf8",
    maxBuffer: MAX_BUFFER,
  });
}

function parseCheckPayload(text) {
  const payload = text.trim();
  if (!payload) {
    return null;
  }

  try {
    const parsed = JSON.parse(payload);
    if (typeof parsed.ok === "boolean" && Array.isArray(parsed.errors)) {
      return parsed;
    }
  } catch (_) {
    return null;
  }

  return null;
}

function normalizeCheckResult(result, snapshotPath, filePath) {
  return {
    ...result,
    errors: result.errors.map((error) => ({
      ...error,
      file: error.file === snapshotPath ? filePath : error.file,
    })),
  };
}

async function withSnapshot(options, action) {
  const snapshotPath = snapshotFilePath(options.filePath);
  await fs.writeFile(snapshotPath, options.text, "utf8");
  try {
    return await action(snapshotPath);
  } finally {
    await fs.unlink(snapshotPath).catch(() => {});
  }
}

function snapshotFilePath(filePath) {
  const resolvedPath = filePath || path.join(os.tmpdir(), "untitled.nod");
  const ext = path.extname(resolvedPath) || ".nod";
  const base = path.basename(resolvedPath, ext) || "untitled";
  const dir = path.dirname(resolvedPath);
  const stamp = `${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return path.join(dir, `.${base}.nodia-${stamp}${ext}`);
}

function resolveExecutable(configuredPath, filePath, workspacePath) {
  if (configuredPath && configuredPath.trim() !== "") {
    return configuredPath.trim();
  }

  const binaryName = process.platform === "win32" ? "nodia.exe" : "nodia";
  const candidates = [];
  const roots = [workspacePath, filePath ? path.dirname(filePath) : null].filter(Boolean);

  for (const root of roots) {
    candidates.push(path.join(root, "target", "debug", binaryName));
    candidates.push(path.join(root, "target", "release", binaryName));
  }

  for (const candidate of candidates) {
    try {
      require("fs").accessSync(candidate);
      return candidate;
    } catch (_) {
      continue;
    }
  }

  return binaryName;
}

module.exports = {
  checkSnapshot,
  formatSnapshot,
  resolveExecutable,
};
