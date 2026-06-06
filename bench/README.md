# Text Workflow Benchmarks

This directory holds the `0.6.5` baseline for representative whole-text
workflows.

Run the harness with:

```bash
make bench
```

Or directly:

```bash
RUNS=5 sh bench/text-workflows.sh
```

The harness:

* ensures a release binary is available;
* generates deterministic messy-text fixtures under a temporary directory;
* runs each workflow multiple times;
* prints a compact TSV summary with input size, output size, best run, and
  average run time.

Current workflows:

* `normalize-messy-text` — trims, filters, and whitespace-normalizes noisy
  line-oriented input.
* `extract-urls` — runs a whole-text regex extraction pass with named captures.
* `summarize-audit-log` — parses noisy log lines and aggregates action counts.
