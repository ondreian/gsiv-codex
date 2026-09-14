#!/usr/bin/env bash
# Apply the branch ruleset in .github/ruleset-main.json.
#
# The configuration is committed and reviewed; this only installs it. Rerunning
# updates the existing ruleset rather than making a second one, so the file is
# the state and GitHub is the copy.
#
# Why not Terraform: this is one ruleset on one repository. Terraform would add
# a provider, a lock file, a token with repo-admin scope, and a state file that
# has to live somewhere -- to manage twenty lines of JSON that the API takes
# verbatim. If this grows to several repositories with shared settings, the
# JSON below maps one-to-one onto `github_repository_ruleset` and the move is
# mechanical.
set -euo pipefail
cd "$(dirname "$0")/.."

repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
file=.github/ruleset-main.json
name=$(jq -r .name "$file")

existing=$(gh api "repos/$repo/rulesets" --jq ".[] | select(.name == \"$name\") | .id" | head -1)
if [ -n "$existing" ]; then
  gh api -X PUT "repos/$repo/rulesets/$existing" --input "$file" >/dev/null
  echo "updated ruleset $name ($existing) on $repo"
else
  id=$(gh api -X POST "repos/$repo/rulesets" --input "$file" --jq .id)
  echo "created ruleset $name ($id) on $repo"
fi

gh api "repos/$repo/rules/branches/$(gh repo view --json defaultBranchRef --jq .defaultBranchRef.name)" \
  --jq 'group_by(.type) | map({(.[0].type): length}) | add'
