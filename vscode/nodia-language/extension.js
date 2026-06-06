"use strict";

const vscode = require("vscode");
const path = require("path");
const {
  KEYWORDS,
  KEYWORD_SNIPPETS,
  REGEX_DSL_ITEMS,
  REGEX_FLAGS,
  STDLIB_MODULES,
  detectContext,
  parseStdlibUses
} = require("./src/completion");
const {
  checkSnapshot,
  formatSnapshot,
  resolveExecutable
} = require("./src/tooling");

const CHECK_SOURCE = "nodia check";
const DEFAULT_CHECK_DELAY_MS = 250;

function activate(context) {
  const selector = { language: "nodia", scheme: "*" };
  const diagnostics = vscode.languages.createDiagnosticCollection("nodia");
  const pendingChecks = new Map();
  const checkTickets = new Map();
  const shownErrors = new Set();

  const provider = vscode.languages.registerCompletionItemProvider(
    selector,
    {
      provideCompletionItems(document, position) {
        const documentText = document.getText();
        const textBeforeCursor = document.getText(new vscode.Range(new vscode.Position(0, 0), position));
        const linePrefix = document.lineAt(position).text.slice(0, position.character);
        const imports = parseStdlibUses(documentText);
        const completionContext = detectContext(documentText, textBeforeCursor, linePrefix);

        switch (completionContext.kind) {
          case "member":
            return memberItems(completionContext.alias, imports);
          case "use-module":
            return stdlibModuleItems();
          case "use-members":
            return useMemberItems(completionContext.moduleName, completionContext.entered);
          case "use-clauses":
            return useClauseItems(completionContext.moduleName);
          case "regex-dsl":
            return regexDslItems();
          case "general":
            return generalItems();
          default:
            return [];
        }
      }
    },
    ".",
    " ",
    ","
  );

  const onWillSave = vscode.workspace.onWillSaveTextDocument((event) => {
    if (!isFileBackedNodiaDocument(event.document) || !settings().formatOnSave) {
      return;
    }
    event.waitUntil(formatEditsForSave(event.document));
  });

  const onDidOpen = vscode.workspace.onDidOpenTextDocument((document) => {
    scheduleCheck(document, 0);
  });

  const onDidChange = vscode.workspace.onDidChangeTextDocument((event) => {
    if (event.contentChanges.length === 0) {
      return;
    }
    scheduleCheck(event.document);
  });

  const onDidSave = vscode.workspace.onDidSaveTextDocument((document) => {
    scheduleCheck(document, 0);
  });

  const onDidChangeConfiguration = vscode.workspace.onDidChangeConfiguration((event) => {
    if (!event.affectsConfiguration("nodia")) {
      return;
    }
    shownErrors.clear();
    for (const document of vscode.workspace.textDocuments) {
      scheduleCheck(document, 0);
    }
  });

  const onDidClose = vscode.workspace.onDidCloseTextDocument((document) => {
    clearPendingCheck(document);
    diagnostics.delete(document.uri);
  });

  context.subscriptions.push(
    provider,
    diagnostics,
    onWillSave,
    onDidOpen,
    onDidChange,
    onDidSave,
    onDidChangeConfiguration,
    onDidClose,
  );

  for (const document of vscode.workspace.textDocuments) {
    scheduleCheck(document, 0);
  }

  function settings() {
    const configuration = vscode.workspace.getConfiguration("nodia");
    return {
      checkerDelayMs: Math.max(0, configuration.get("checkerDelayMs", DEFAULT_CHECK_DELAY_MS)),
      enableChecker: configuration.get("enableChecker", true),
      executablePath: configuration.get("executablePath", ""),
      formatOnSave: configuration.get("formatOnSave", true),
    };
  }

  function isFileBackedNodiaDocument(document) {
    return document.languageId === "nodia" && document.uri.scheme === "file";
  }

  function scheduleCheck(document, delayMs = null) {
    if (!isFileBackedNodiaDocument(document)) {
      return;
    }
    if (!settings().enableChecker) {
      clearPendingCheck(document);
      diagnostics.delete(document.uri);
      return;
    }

    clearPendingCheck(document);
    const timeout = setTimeout(() => {
      pendingChecks.delete(document.uri.toString());
      void updateDiagnostics(document);
    }, delayMs ?? settings().checkerDelayMs);
    pendingChecks.set(document.uri.toString(), timeout);
  }

  function clearPendingCheck(document) {
    const key = document.uri.toString();
    const timeout = pendingChecks.get(key);
    if (timeout) {
      clearTimeout(timeout);
      pendingChecks.delete(key);
    }
  }

  async function updateDiagnostics(document) {
    const key = document.uri.toString();
    const ticket = (checkTickets.get(key) || 0) + 1;
    checkTickets.set(key, ticket);

    try {
      const result = await checkSnapshot(toolingOptions(document));
      if (checkTickets.get(key) !== ticket) {
        return;
      }

      const items = result.errors
        .filter((error) => !error.file || error.file === document.uri.fsPath)
        .map((error) => toDiagnostic(document, error));
      diagnostics.set(document.uri, items);
    } catch (error) {
      if (checkTickets.get(key) !== ticket) {
        return;
      }
      diagnostics.delete(document.uri);
      handleIntegrationFailure("check", error);
    }
  }

  async function formatEditsForSave(document) {
    try {
      const formatted = await formatSnapshot(toolingOptions(document));
      if (formatted === document.getText()) {
        return [];
      }
      return [vscode.TextEdit.replace(fullDocumentRange(document), formatted)];
    } catch (error) {
      handleFormatFailure(error);
      return [];
    }
  }

  function toolingOptions(document) {
    const currentSettings = settings();
    return {
      cwd: path.dirname(document.uri.fsPath),
      executable: resolveExecutable(
        currentSettings.executablePath,
        document.uri.fsPath,
        workspacePath(document),
      ),
      filePath: document.uri.fsPath,
      text: document.getText(),
    };
  }

  function workspacePath(document) {
    return vscode.workspace.getWorkspaceFolder(document.uri)?.uri.fsPath || path.dirname(document.uri.fsPath);
  }

  function fullDocumentRange(document) {
    const lastLine = Math.max(0, document.lineCount - 1);
    return new vscode.Range(
      new vscode.Position(0, 0),
      document.lineAt(lastLine).range.end,
    );
  }

  function toDiagnostic(document, error) {
    const line = clamp((error.line || 1) - 1, 0, Math.max(0, document.lineCount - 1));
    const lineText = document.lineAt(line).text;
    const column = clamp((error.column || 1) - 1, 0, lineText.length);
    const endColumn = column < lineText.length ? column + 1 : column;
    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(line, column, line, endColumn),
      error.message,
      vscode.DiagnosticSeverity.Error,
    );
    diagnostic.code = error.code;
    diagnostic.source = CHECK_SOURCE;
    return diagnostic;
  }

  function clamp(value, min, max) {
    return Math.min(Math.max(value, min), max);
  }

  function handleFormatFailure(error) {
    if (error && error.code === "ENOENT") {
      showErrorOnce(
        "Cannot run Nodia formatter. Configure 'nodia.executablePath' or make 'nodia' available in PATH.",
      );
      return;
    }

    const message = extractToolMessage(error);
    if (message.startsWith("error[")) {
      return;
    }
  }

  function handleIntegrationFailure(toolName, error) {
    if (error && error.code === "ENOENT") {
      showErrorOnce(
        `Cannot run Nodia ${toolName}. Configure 'nodia.executablePath' or make 'nodia' available in PATH.`,
      );
      return;
    }

    const message = extractToolMessage(error);
    if (!message.startsWith("error[")) {
      console.error(`[nodia] ${toolName} integration failed: ${message}`);
    }
  }

  function extractToolMessage(error) {
    if (!error) {
      return "unknown error";
    }
    if (typeof error.message === "string" && error.message.trim() !== "") {
      return error.message.trim();
    }
    if (typeof error.stderr === "string" && error.stderr.trim() !== "") {
      return error.stderr.trim();
    }
    if (typeof error.stdout === "string" && error.stdout.trim() !== "") {
      return error.stdout.trim();
    }
    return String(error);
  }

  function showErrorOnce(message) {
    if (shownErrors.has(message)) {
      return;
    }
    shownErrors.add(message);
    void vscode.window.showErrorMessage(message);
  }
}

function deactivate() {}

function stdlibModuleItems() {
  return Object.entries(STDLIB_MODULES).map(([name, spec]) => {
    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Module);
    item.detail = spec.summary;
    item.documentation = new vscode.MarkdownString(`Stdlib namespace \`${name}\`.`);
    item.sortText = `010-${name}`;
    return item;
  });
}

function useClauseItems(moduleName) {
  const items = [];
  items.push(keywordItem("as", "Namespace alias", "010"));
  items.push(keywordItem("pick", "Limit visible names inside the namespace", "020"));
  items.push(keywordItem("hide", "Exclude names inside the namespace", "030"));
  return items;
}

function useMemberItems(moduleName, enteredNames) {
  const entered = new Set(enteredNames);
  return Object.entries(STDLIB_MODULES[moduleName].members)
    .filter(([name]) => !entered.has(name))
    .map(([name, arities]) => callableItem(name, moduleName, arities, "010"));
}

function memberItems(alias, imports) {
  const imported = imports.get(alias);
  if (!imported) {
    return [];
  }

  return imported.members.map((memberName) => {
    const arities = STDLIB_MODULES[imported.moduleName].members[memberName];
    return callableItem(memberName, imported.moduleName, arities, "010");
  });
}

function generalItems() {
  const items = [];

  for (const snippet of KEYWORD_SNIPPETS) {
    items.push(snippetItem(snippet.label, snippet.insertText, snippet.detail, "100"));
  }

  for (const keyword of KEYWORDS) {
    items.push(keywordItem(keyword, "Language keyword", "200"));
  }

  items.push(constantItem("input", "CLI input map", "090"));
  return dedupeItems(items);
}

function regexDslItems() {
  const items = REGEX_DSL_ITEMS.map((entry) =>
    entry.insertText
      ? snippetItem(entry.label, entry.insertText, entry.detail, "050")
      : keywordItem(entry.label, entry.detail, "050")
  );

  for (const flag of REGEX_FLAGS) {
    items.push(keywordItem(flag, "Regex flag", "060"));
  }

  return dedupeItems(items);
}

function callableItem(name, moduleName, arities, sortText) {
  if (arities === null) {
    return constantItem(name, `${moduleName} binding`, sortText);
  }

  const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Function);
  item.detail = signatureSummary(moduleName, name, arities);
  item.documentation = new vscode.MarkdownString(
    `Stdlib callable from \`${moduleName}\`.\n\nAllowed arities: ${arities.join(", ")}.`
  );
  item.sortText = `${sortText}-${name}`;
  return item;
}

function constantItem(name, detail, sortText) {
  const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Constant);
  item.detail = detail;
  item.sortText = `${sortText}-${name}`;
  return item;
}

function keywordItem(label, detail, sortText) {
  const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Keyword);
  item.detail = detail;
  item.sortText = `${sortText}-${label}`;
  return item;
}

function snippetItem(label, insertText, detail, sortText) {
  const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
  item.insertText = new vscode.SnippetString(insertText);
  item.detail = detail;
  item.sortText = `${sortText}-${label}`;
  return item;
}

function signatureSummary(moduleName, name, arities) {
  if (arities.length === 1) {
    const arity = arities[0];
    return `${moduleName}.${name} (${arity} arg${arity === 1 ? "" : "s"})`;
  }
  return `${moduleName}.${name} (${arities.join(" or ")} args)`;
}

function dedupeItems(items) {
  const seen = new Set();
  return items.filter((item) => {
    const key = item.label;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

module.exports = {
  activate,
  deactivate
};
