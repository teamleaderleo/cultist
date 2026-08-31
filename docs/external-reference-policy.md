# External GitHub reference policy

Cultist studies public GitHub history heavily. Internal research can churn. Third-party interaction text must stay quiet unless a human deliberately chooses otherwise.

The prevention boundary is **before the GitHub write**. A CI job that inspects a pull-request body runs after GitHub has already processed that body, so CI is only a detector and cleanup aid.

## Automated-worker invariant

Every third-party GitHub reference an automated worker creates in human-facing interaction text must already be backlink-safe before the write occurs.

Default to non-linking wording:

```text
OWNER/REPOSITORY issue 123
OWNER/REPOSITORY PR 456
OWNER/REPOSITORY discussion 789
```

If click-through is useful, use the literal redirect host:

```text
https://redirect.github.com/OWNER/REPOSITORY/issues/123
https://redirect.github.com/OWNER/REPOSITORY/pull/456
https://redirect.github.com/OWNER/REPOSITORY/discussions/789
https://redirect.github.com/OWNER/REPOSITORY/commit/SHA
```

Automated interaction text must not contain:

- direct third-party `github.com` URLs;
- third-party `OWNER/REPOSITORY#123` shorthand;
- an intentional-evidence marker used to bypass the interaction rule.

Repositories under `teamleaderleo/*` are first-party coordination surfaces for this policy. Ordinary links and shorthand among those repositories are allowed.

## Interaction preflight

Before an automated worker creates or edits an issue, pull request, comment, review, inline review comment, or discussion that mentions third-party GitHub work, run the scanner against the **exact text that will be written**:

```sh
python3 scripts/external_github_reference_guard.py \
  --repository teamleaderleo/cultist \
  --stdin < proposed-body.md
```

The GitHub write happens only after this command succeeds.

Interaction preflight is intentionally stricter than repository-file detection:

- it scans direct third-party URLs even inside code blocks;
- it rejects third-party cross-repository shorthand;
- it has no automated evidence-marker exception.

A post-write workflow cannot undo a backlink that GitHub has already emitted.

## Commit messages

Do not put clickable third-party issue/PR/discussion references, direct URLs, or `OWNER/REPOSITORY#123` shorthand in automated commit messages.

Use plain non-linking wording such as `ProjectName PR 123`, or omit the external coordinate from the commit message and preserve it in the research receipt instead.

## Repository files and machine evidence

Ordinary tracked files do not create issue/PR conversation backlinks the same way GitHub interaction text does, but automated workers should still use one consistent presentation rule: non-linking wording by default, redirect links when click-through helps.

Canonical provider coordinates may remain exact where identity requires them:

```text
https://github.com/...
https://api.github.com/...
```

Examples include:

- GitHub API requests;
- exact provider URLs retained in JSON receipts;
- parser fixtures whose purpose is to recognize canonical GitHub syntax;
- workflow inputs consumed by GitHub tooling;
- source payloads copied verbatim as evidence.

For a rare exact canonical evidence line in changed Markdown, the changed-file detector still accepts one local marker:

```html
<!-- cultist:allow-canonical-github-evidence -->
```

That marker applies only to repository-file detection. It never exempts automated interaction preflight.

## CI detector

CI scans newly added Markdown lines and pull-request bodies after the write. Existing historical Markdown is not rescanned on unrelated changes.

The CI detector is useful for catching drift and cleaning up presentation. It is not the mechanism that prevents a third-party backlink.

## Rule of thumb

```text
third-party human-facing mention
  OWNER/REPOSITORY issue|PR|discussion NUMBER

third-party human-facing link
  redirect.github.com

owned teamleaderleo/* reference
  ordinary GitHub reference allowed

provider identity / machine input
  canonical URL when exact identity requires it

GitHub interaction write
  exact text passes --stdin preflight first
```

The goal is high-volume internal research without high-volume third-party GitHub cross-reference noise.
