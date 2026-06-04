"use strict";

const vscode = require("vscode");
const {
  KEYWORDS,
  KEYWORD_SNIPPETS,
  REGEX_DSL_ITEMS,
  REGEX_FLAGS,
  STDLIB_MODULES,
  detectContext,
  parseStdlibUses
} = require("./src/completion");

function activate(context) {
  const selector = { language: "nodia", scheme: "*" };
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

  context.subscriptions.push(provider);
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
