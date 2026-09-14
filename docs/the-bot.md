# urnon-bot

A GitHub App owned by a personal account, so the release pull request is opened
by something other than the workflow itself.

## Why not just a token

`GITHUB_TOKEN` can open the release pull request and deliberately cannot make
it do anything: a workflow run triggered by one is held at `action_required` so
a workflow cannot start itself in a loop. CI is a required check on `main`, so
the release then waits on a click for a reason unrelated to whether the release
is any good.

A personal access token solves that and costs more than it looks:

| | personal access token | GitHub App |
| --- | --- | --- |
| acts as | you | the app |
| reach | every repository you can write to | only where it is installed |
| token lifetime | until rotated | one hour, minted per run |
| when it expires | CI breaks, on a day nobody chose | never; the key is long-lived, the tokens are not |
| revoking it | rotates everything it was used for | uninstall, and only this stops |

## The one that exists

`urnon-bot`, app id **4946103**, owned by `ondreian`, installed on `gsiv-codex`
and `urnon` — [github.com/apps/urnon-bot](https://github.com/apps/urnon-bot).
Its id is in the repository variable `BOT_APP_ID` and its key in the secret
`BOT_PRIVATE_KEY`.

It holds exactly two permissions: contents, and pull requests. It cannot merge
anything, and it is not a bypass actor on the branch ruleset.

## Making another

Ten minutes, once. It belongs to a personal account — no organisation needed.

1. **github.com/settings/apps/new**
   - name: `urnon-bot` (names are global; add a suffix if taken)
   - homepage: this repository
   - **uncheck Webhook → Active.** There is no server to call.
   - Repository permissions: **Contents: read & write**, **Pull requests: read
     & write**. Nothing else — that is the whole point.
   - "Only on this account".
2. **Create**, then on the app's page: **Generate a private key**. A `.pem`
   downloads. That file is the credential; there is no second copy.
3. **Install App** → this repository (or "all repositories", your call).
4. Note the **App ID** from the app's page.

Then give the repository both halves:

```sh
gh variable set BOT_APP_ID --body 123456
gh secret   set BOT_PRIVATE_KEY < ~/Downloads/urnon-bot.*.private-key.pem
rm ~/Downloads/urnon-bot.*.private-key.pem
```

The app id is a variable rather than a secret because it is not one — it is on
the app's public page, and masking it only makes logs harder to read.

## What changes

Nothing, until both exist. `release-please.yml` asks for an installation token
and lets the step fail; with no app configured it falls back to `github.token`
and behaves exactly as before, click and all. With the app it opens the release
pull request as `urnon-bot`, CI runs on its own, and the release waits on
nobody.

## What it is not for

Merging. The app opens and updates the release pull request; a person still
decides that a release happens. Do not add the app as a bypass actor on the
branch ruleset — a bot that can push to `main` is a bot that can push anything
to `main`.
