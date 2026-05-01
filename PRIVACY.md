# Privacy Policy

**Last updated:** April 30, 2026

## What inspect-review does

inspect-review is a GitHub App that performs entity-level code review on pull requests. When triggered, it reads the PR diff and file contents, analyzes them using LLM APIs (OpenAI, Anthropic), and posts review comments.

## Data we access

- Repository file contents and diffs (only for PRs where inspect is triggered)
- PR metadata (title, description, author, file list)
- GitHub webhook payloads (repository name, installation info)

## Data we store

We do not store your code. File contents and diffs are held in memory during review and discarded after the review completes. No code is written to disk or persisted in any database.

We retain basic usage logs (repository name, PR number, timestamp, review status) for debugging and reliability purposes.

## Third-party services

Code snippets from PR diffs are sent to LLM APIs (OpenAI and Anthropic) for analysis. These providers process the data under their respective API terms, which prohibit using API inputs for model training.

No other third parties receive your data.

## Data security

- All communication uses TLS
- GitHub webhook payloads are verified using HMAC-SHA256 signatures
- GitHub App installation tokens are short-lived and scoped to installed repositories
- Infrastructure runs on Fly.io (SOC 2 compliant)

## Your rights

You can uninstall the GitHub App at any time. Uninstalling stops all data access immediately. Since we don't persist code, there is nothing to delete.

## Contact

Questions about this policy: rs545837@gmail.com
