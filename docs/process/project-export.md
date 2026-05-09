# Project Export

Status: local contributor utility

The project export helper creates a single text file, `project-export.txt`, that
contains the repository's text files with clear file separators. It is intended
for situations where another review or design tool needs broad code context but
cannot directly inspect the local workspace.

Run it from the repository root:

```bash
npm run export
```

The generated `project-export.txt` is ignored by git. Re-run the command when
you need a fresh snapshot.

## Why Use It

The export is useful when you want an outside AI tool to analyze the codebase,
explain architecture, suggest UI directions, or generate visual/product ideas
from the real code instead of a tiny copied snippet.

One practical workflow is:

1. Run `npm run export`.
2. Give `project-export.txt` to a tool such as ChatGPT.
3. Ask it to analyze the current code and generate UI or product ideas from the
   actual implementation context.
4. Bring the useful ideas back into Ordo as normal issues, docs, or pull
   requests with evidence and review.

This can make invisible backend work easier to reason about visually. For
example, after adding backend read models, the export can help another tool see
the route contracts and sketch UI possibilities that are not obvious from the
code alone.

## Safety Boundary

The export helper is conservative by default. It skips common dependency,
build, runtime, binary, and secret-bearing files, including:

- `.env*` files;
- `.data`, `.runtime-logs`, `.ordo`, and `.ordo-artifacts`;
- `.git`, `.next`, `node_modules`, `target`, `coverage`, and test reports;
- private local docs folders matching `docs/_*/`;
- database, key, log, image, PDF, and similar binary-looking file extensions.

Still review `project-export.txt` before sharing it outside your machine. The
script avoids obvious secrets and generated data, but it cannot understand every
future file's sensitivity.

## What It Is Not

The export is not release evidence, not a source of truth, and not a substitute
for review. It is a portable context bundle for analysis. Durable project truth
still lives in the repository, docs, issues, pull requests, tests, and runtime
evidence.