"use strict";

const {
  KEYWORDS,
  KEYWORD_SNIPPETS,
  REGEX_DSL_ITEMS,
  REGEX_FLAGS,
  STDLIB_MODULES,
  moduleNames
} = require("./catalog");

const IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]*$/;
const STDLIB_NAME_SET = new Set(moduleNames());

function parseStdlibUses(source) {
  const imports = new Map();
  const lines = source.split(/\r?\n/);

  for (const line of lines) {
    const match = line.match(/^\s*use\s+([A-Za-z_][A-Za-z0-9_]*)(.*)$/);
    if (!match) {
      continue;
    }

    const moduleName = match[1];
    if (!STDLIB_NAME_SET.has(moduleName)) {
      continue;
    }

    const clauses = parseUseClauses(match[2] || "");
    const alias = clauses.alias || moduleName;
    imports.set(alias, {
      alias,
      hidden: clauses.hide,
      members: selectedMembers(moduleName, clauses.pick, clauses.hide),
      moduleName,
      pick: clauses.pick
    });
  }

  return imports;
}

function parseUseClauses(text) {
  const tokens = text.match(/[A-Za-z_][A-Za-z0-9_]*|,/g) || [];
  const clauses = {
    alias: null,
    hide: [],
    pick: []
  };

  let index = 0;
  while (index < tokens.length) {
    const token = tokens[index];
    index += 1;

    if (token === "as") {
      const alias = tokens[index];
      if (alias && IDENTIFIER.test(alias)) {
        clauses.alias = alias;
        index += 1;
      }
      continue;
    }

    if (token === "pick" || token === "hide") {
      const key = token;
      while (index < tokens.length) {
        const value = tokens[index];
        if (value === "as" || value === "pick" || value === "hide") {
          break;
        }
        if (value !== ",") {
          clauses[key].push(value);
        }
        index += 1;
      }
    }
  }

  return clauses;
}

function selectedMembers(moduleName, pick, hide) {
  const allMembers = Object.keys(STDLIB_MODULES[moduleName].members);
  const pickedMembers = pick.length === 0 ? allMembers : pick.filter((name) => allMembers.includes(name));
  return pickedMembers.filter((name) => !hide.includes(name));
}

function detectContext(documentText, textBeforeCursor, linePrefix) {
  if (isCommentLine(linePrefix)) {
    return { kind: "none" };
  }

  const member = linePrefix.match(/([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z0-9_]*)?$/);
  if (member) {
    return {
      alias: member[1],
      kind: "member",
      prefix: member[2] || ""
    };
  }

  const useModule = linePrefix.match(/^\s*use\s+([A-Za-z_]*)$/);
  if (useModule) {
    return {
      kind: "use-module",
      prefix: useModule[1] || ""
    };
  }

  const useClause = linePrefix.match(
    /^\s*use\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+as\s+[A-Za-z_][A-Za-z0-9_]*)?(?:\s+(pick|hide)\s+([A-Za-z0-9_,\s]*))?\s*$/
  );
  if (useClause && STDLIB_NAME_SET.has(useClause[1])) {
    if (useClause[2]) {
      return {
        entered: parseClauseNames(useClause[3] || ""),
        kind: "use-members",
        moduleName: useClause[1],
        prefix: currentClausePrefix(useClause[3] || "")
      };
    }
    return {
      kind: "use-clauses",
      moduleName: useClause[1]
    };
  }

  if (isProbablyInsideRegexDsl(documentText, textBeforeCursor)) {
    return { kind: "regex-dsl" };
  }

  return { kind: "general" };
}

function parseClauseNames(text) {
  return (text.match(/[A-Za-z_][A-Za-z0-9_]*/g) || []).filter((name) => name !== "pick" && name !== "hide" && name !== "as");
}

function currentClausePrefix(text) {
  const match = text.match(/([A-Za-z_][A-Za-z0-9_]*)?$/);
  return match ? match[1] || "" : "";
}

function isProbablyInsideRegexDsl(documentText, textBeforeCursor) {
  const lookback = documentText.slice(Math.max(0, textBeforeCursor.length - 4000), textBeforeCursor.length);
  const stack = [];
  let index = 0;

  while (index < lookback.length) {
    if (lookback[index] === "#" || (lookback[index] === "/" && lookback[index + 1] === "/")) {
      while (index < lookback.length && lookback[index] !== "\n") {
        index += 1;
      }
      continue;
    }

    const regexStart = lookback.slice(index).match(/^regex\b(?:\s*\([^)]*\))?\s*\{/);
    if (regexStart) {
      stack.push("regex");
      index += regexStart[0].length;
      continue;
    }

    const rawTriple = lookback.slice(index).match(/^r?"""/);
    if (rawTriple) {
      const delimiter = rawTriple[0];
      index += delimiter.length;
      const end = lookback.indexOf('"""', index);
      index = end === -1 ? lookback.length : end + 3;
      continue;
    }

    const rawSingle = lookback.slice(index).match(/^r'/);
    if (rawSingle) {
      index += 2;
      while (index < lookback.length && lookback[index] !== "'") {
        index += 1;
      }
      index += 1;
      continue;
    }

    const rawDouble = lookback.slice(index).match(/^r"/);
    if (rawDouble) {
      index += 2;
      while (index < lookback.length && lookback[index] !== '"') {
        index += 1;
      }
      index += 1;
      continue;
    }

    if (lookback[index] === "'" || lookback[index] === '"') {
      const delimiter = lookback[index];
      index += 1;
      while (index < lookback.length) {
        if (lookback[index] === "\\") {
          index += 2;
          continue;
        }
        if (lookback[index] === delimiter) {
          index += 1;
          break;
        }
        index += 1;
      }
      continue;
    }

    if (lookback[index] === "{") {
      stack.push("block");
      index += 1;
      continue;
    }

    if (lookback[index] === "}") {
      stack.pop();
      index += 1;
      continue;
    }

    index += 1;
  }

  return stack.includes("regex");
}

function isCommentLine(linePrefix) {
  const trimmed = linePrefix.trimStart();
  return trimmed.startsWith("#") || trimmed.startsWith("//");
}

module.exports = {
  KEYWORDS,
  KEYWORD_SNIPPETS,
  REGEX_DSL_ITEMS,
  REGEX_FLAGS,
  STDLIB_MODULES,
  detectContext,
  parseStdlibUses,
  selectedMembers
};
