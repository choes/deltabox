---
name: deltabox
description: Use when a user asks an agent to find, inspect, tag, or reason about files stored in a local deltabox vault through deltabox-cli. This skill covers local JSON-based workflows for search, metadata lookup, text segment review, tags, and storage locations.
---

# deltabox

Use `deltabox-cli` through the `deltabox` binary to work with a local deltabox vault. Prefer JSON output whenever available so results are stable to parse.

## Preconditions

- Ask for or infer the vault path before running commands. Use `--vault <path>`.
- If the binary is not on `PATH`, use `cargo run -q -p deltabox-cli -- --vault <path> ...` from the repository root.
- Default to read-only actions. Do not tag, delete, purge, move storage, or change policies unless the user explicitly asks.

## Core Commands

Search with explanations:

```bash
deltabox --vault <vault> search "<query>" --details --json
```

Read file metadata:

```bash
deltabox --vault <vault> info <file_id> --json
```

Read tags for a file:

```bash
deltabox --vault <vault> tag file <file_id> --json
```

Read storage locations:

```bash
deltabox --vault <vault> storage locations <file_id> --json
```

Attach a user-requested tag:

```bash
deltabox --vault <vault> tag attach <file_id> "<tag>"
```

## Search Workflow

1. Run `search "<query>" --details --json`.
2. Prefer results with text matches over name/path-only matches when answering content questions.
3. Cite `logical_path`, `file_id`, and relevant match details.
4. For PDF matches, include `page` when present.
5. For text matches, include `line_start` / `line_end` when present.
6. If results are broad, refine with more specific terms from the user's request.

## Answering Rules

- Summarize only the matching snippets and metadata returned by deltabox unless the user asks to restore or open the file.
- Do not claim a file is the correct one solely from filename if detailed matches are available.
- If no results are found, say that clearly and suggest a narrower or alternative query.
- Do not expose backend credentials or secret config values.
- Before destructive operations such as `delete`, `purge`, `storage remove-location`, or `storage move`, confirm the exact `file_id` and path with the user.

## Useful Output Fields

- Search result: `file.file_id`, `file.logical_path`, `file.name`, `file.size`, `matches`.
- Match: `match_kind`, `source`, `text`, `page`, `line_start`, `line_end`.
- Storage location: `chunk_id`, `backend_id`, `object_key`, `status`.
- Tag: `tag_id`, `name`, `tag_type`, `source`.
