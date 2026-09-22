# GitHub connector: execution index

**Attach this file to a future chat when asking the assistant to publish or edit a repository.**
Schema snapshot: **2026-09-21**. Source: live `api_tool.list_resources` descriptions for `GitHub`. Tool schemas were inspected; no repository writes were executed to produce this index. Rediscover tools in each session; current contracts and permissions take precedence.

## 1. Start here: discover, then act

These are assistant tool calls, not shell commands. Connector discovery and GitHub calls use the **commentary** channel. Load the relevant schemas, then invoke `GitHub.<action>` directly.

```text
api_tool.list_resources({"paths":["GitHub"],"query":"create"})
api_tool.list_resources({"paths":["GitHub"],"query":"ref"})
api_tool.list_resources({"paths":["GitHub"],"query":"get_user_login"})
```

The inspected results expose identity/repository reads, file writes, Git-object writes, branch updates, and pull requests. Discovery results may overlap. If a filtered query returns nothing or appears incomplete, retry **without a filter** before concluding that an action is unavailable:

```text
api_tool.list_resources({"paths":["GitHub"]})
```

**Do not equate repository creation with repository editing.** No repository-creation action appeared in the inspected `create` results. That does not make the connector read-only. Recheck the current catalog before reporting a missing capability. Lack of a local `gh` executable is not a connector limitation. Discovery alone does not establish that a write succeeded or that this connection can write to the requested repository.

## 2. Resolve identity and the actual repository

Replace uppercase placeholders with observed values; do not send them literally.

```text
GitHub.get_user_login({})
GitHub.get_repo({"repository_url":"https://github.com/OWNER/REPO"})
```

`get_repo` accepts **exactly one** of `repository_url`, `repository_full_name`, or `repository_id`. Use its result to establish the canonical repository and default branch. Do not infer the owner from the ChatGPT display name or a remembered GitHub login.

For optional permission discovery, load its schema first:

```text
api_tool.list_resources({"paths":["GitHub"],"query":"get_repo_collaborator_permission"})
GitHub.get_repo_collaborator_permission({"repository_full_name":"OWNER/REPO","username":"OBSERVED_LOGIN"})
```

Collaborator permissions alone do not prove the connector's token/app has equivalent access. A failed permission check is not proof that all writes are unavailable. An empty repository-search result is not conclusive either: try `get_repo` against the user-supplied URL. Report concrete errors and distinguish missing tools, installation access, token permissions, branch rules, and missing repositories. Never bypass protections.

## 3. Exact read and single-file write calls

Fetch the existing file **on the branch being edited**:

```text
GitHub.fetch_file({"repository_full_name":"OWNER/REPO","path":"README.md","ref":"BRANCH"})
```

Read the complete file before replacing it; a line-limited or truncated response is not the full content. The file's returned `sha` is a **blob SHA**, not a commit SHA.

Create a path that does not exist:

```text
GitHub.create_file({"repository_full_name":"OWNER/REPO","path":"docs/example.md","content":"# Example\n","message":"Add example documentation","branch":"BRANCH"})
```

Replace an existing path, using its current blob SHA:

```text
GitHub.update_file({"repository_full_name":"OWNER/REPO","path":"README.md","content":"COMPLETE_REPLACEMENT_TEXT","message":"Update README","sha":"CURRENT_FILE_BLOB_SHA","branch":"BRANCH"})
```

`create_file` and `update_file` accept **plain UTF-8 text**, not a local path, patch, or base64-encoded text; the wrapper handles encoding. They create commits immediately. `create_file` returns the resulting commit SHA; `update_file` returns commit and content-blob SHAs. Use the returned `content_sha` for a subsequent sequential update. Do not run conflicting update/delete operations on the same path in parallel. `create_file` does not create a requested branch.

## 4. Multi-file import: one commit, then a pull request

Use this route for a source ZIP or changes that should land together, rather than creating one commit per file. It requires an **existing initial commit**. The example uses a new review branch; substitute the user's authorized workflow rather than assuming permission to push to the default branch.

### A. Read the base branch and its tree

```text
GitHub.fetch({"url":"https://api.github.com/repos/OWNER/REPO/git/ref/heads/BASE_BRANCH"})
GitHub.fetch({"url":"https://api.github.com/repos/OWNER/REPO/git/commits/BASE_COMMIT_SHA"})
```

Take `BASE_COMMIT_SHA` from the first response's `object.sha`, then `BASE_TREE_SHA` from the second response's `tree.sha`. Handle the connector's response envelope; do not assume the raw API object is always top-level. URL-encode dynamic components appropriately.

### B. Create the working branch from that exact commit

```text
GitHub.create_branch({"repository_full_name":"OWNER/REPO","branch_name":"import/codex-accounts","sha":"BASE_COMMIT_SHA"})
```

Supply **exactly one** of `sha` or `base_ref`. Both require an existing commit/ref. If the branch already exists, inspect it; do not reset it automatically.

### C. Create blobs and a preserving tree

For each changed/new file:

```text
GitHub.create_blob({"repository_full_name":"OWNER/REPO","content":"COMPLETE_FILE_TEXT","encoding":"utf-8"})
```

Collect the returned blob SHAs and build one tree:

```text
GitHub.create_tree({"repository_full_name":"OWNER/REPO","base_tree_sha":"BASE_TREE_SHA","tree_elements":[{"path":"README.md","mode":"100644","type":"blob","sha":"README_BLOB_SHA"},{"path":"docs/example.md","mode":"100644","type":"blob","sha":"EXAMPLE_BLOB_SHA"}]})
```

Preserve the base tree and existing file modes. The example uses ordinary non-executable files; executable scripts use `100755`. Do not omit `base_tree_sha` for an ordinary update: a tree built from scratch can remove unrelated tracked paths when committed. Deletions must be intentional and authorized.

### D. Commit, then advance only the intended branch

```text
GitHub.create_commit({"repository_full_name":"OWNER/REPO","message":"Import Codex Accounts source","tree_sha":"NEW_TREE_SHA","parent_sha":"BASE_COMMIT_SHA"})
GitHub.update_ref({"repository_full_name":"OWNER/REPO","branch_name":"import/codex-accounts","sha":"NEW_COMMIT_SHA","force":false})
```

`create_blob`, `create_tree`, and `create_commit` alone do **not** publish changes to a branch. `update_ref` performs that step. Re-read the working branch before advancing it; stop and reconcile unexpected movement. Never resolve a conflict by silently forcing the update. `force:false` is a fast-forward safeguard, not an expected-old-SHA comparison.

### E. Verify and open the PR

```text
GitHub.fetch({"url":"https://api.github.com/repos/OWNER/REPO/git/ref/heads/import/codex-accounts"})
GitHub.fetch_file({"repository_full_name":"OWNER/REPO","path":"README.md","ref":"NEW_COMMIT_SHA"})
GitHub.compare_commits({"repo_full_name":"OWNER/REPO","base":"BASE_COMMIT_SHA","head":"NEW_COMMIT_SHA"})
GitHub.create_pull_request({"repository_full_name":"OWNER/REPO","title":"Import Codex Accounts source","body":"SUMMARY_AND_ACTUAL_VALIDATION","head":"import/codex-accounts","base":"BASE_BRANCH","draft":false})
```

Confirm the ref points to the intended commit and inspect the complete changed-file set and relevant contents. Note that `compare_commits` uses **`repo_full_name`**, while these write calls use **`repository_full_name`**. For the PR, use `head` and `base`; `head_branch`/`base_branch` are compatibility aliases, not additional required fields. Report the actual commit and PR returned. Creating a PR is not merging, releasing, or proving CI passed.

## 5. Boundaries and common traps

- **Truly empty repository:** the exposed `create_commit` requires `parent_sha`; `create_branch` requires an existing commit/ref. Do not fabricate a SHA or assume those calls create a root commit. Check for a supported initialization action in the live catalog. Otherwise, have the owner initialize the repository with a README. Initializing with a README during repository creation avoids this prerequisite. Do not promise that this wrapper's `create_file` bootstraps an empty repository without verifying its supported behavior.
- **Repository creation:** rediscover first. If no action exists, report that specific limitation, not “GitHub is read-only.” Creating an issue, branch, or Git object does not create a repository.
- **ZIP imports:** inspect and extract the source package locally. Tool `content` fields require actual file contents, not `/mnt/data/...` paths. Review hidden files, credentials, licenses, generated outputs, and unexpected paths before uploading. Git-data calls do not apply a local `.gitignore` on your behalf. Do not publish credentials or personal account data.
- **Executables and releases:** `create_blob` supports `encoding:"base64"` for actual binary bytes, but writing a Git blob is not uploading a release asset. Discover a release-upload action separately; do not promise it from the existence of file-write tools. Prefer the source package for a repository import unless the user requests binaries in Git.
- **Generic fetch is read-only:** `GitHub.fetch` supports approved GitHub GET resources. Do not invent POST/PUT arguments or use it for unsupported endpoint families. Use named write actions. Large/truncated responses need bounded follow-up reads; discovery responses can be revisited through `api_tool.read_resource` only when a response resource URI was actually returned.
- **Failure reporting:** cite the failed action and sanitized error, with no tokens. A workflow-file permission error, protected-branch rejection, or installation-access error is a scoped failure, not proof that every connector write is unavailable. Do not claim repository access, commits, uploaded files, or successful tests without returned evidence.

## 6. Optional handoff sentence

> Use the attached GitHub connector index. Rediscover the listed actions, read the target repository, then perform the authorized source import using the connector. Do not substitute a local Git/CLI tutorial unless a specific required connector action or permission actually fails. Do not merge or release unless I request it.
