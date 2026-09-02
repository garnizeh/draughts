# Security Policy

## What this program is

`draughts` is a single binary that plays draughts, runs self-play batches, and
writes commentary with an in-process quantized model. It listens on one HTTP
port, keeps everything it knows in one SQLite file, and has no authentication,
no multi-tenancy, and no notion of a user
([§18](docs/architecture/18-security-and-safety.md)). It is built to be run by
the person who started it, on a machine they control, on an interface they chose
— not to be exposed to the internet.

That is a scope statement, not a disclaimer. A finding is still a finding if it
lets an unauthenticated caller on that port corrupt the database, escape the
process, or make the machine do something the operator did not ask for.

## Supported versions

The latest release, and `main`. There is no maintenance branch and no backport
policy — the project is pre-1.0 and the honest answer is that a fix ships in the
next release.

## Reporting a vulnerability

**Do not open a public issue.** Use GitHub's private reporting:

> **[Report a vulnerability](https://github.com/garnizeh/draughts/security/advisories/new)**
> — the *Security* tab → *Report a vulnerability*.

If that is unavailable to you, email **rodrigobaliza@gmail.com** with `draughts
security` in the subject.

Useful in a report, roughly in order of usefulness:

- What an attacker gets, and what access they needed to get it.
- The smallest reproduction you have — a request, a config, a database file.
- The revision. `draughts --version`, or the commit.
- Whether it reproduces on the portable build, the `cuda` build, or both.

You will get an acknowledgement within **five days** and an assessment within
**fourteen**. If a fix is warranted it ships in the next release, credited in
the [CHANGELOG](CHANGELOG.md) unless you would rather it were not, with a GitHub
Security Advisory published at the same time.

## What is in scope

- Anything reachable through the HTTP surface in
  [§9](docs/architecture/09-api-contract.md) that escapes what that endpoint is
  documented to do.
- Database corruption, or a path that writes a row no `format_version` reader
  can decode ([§13.7](docs/architecture/13-data-dictionary.md)).
- Model output that escapes sanitization and reaches a browser as markup or as a
  terminal control sequence ([§7.7](docs/architecture/07-face-llm-layer.md)).
- Path traversal through configuration, a model path, or the static file
  handler.
- A dependency advisory that `cargo deny check advisories` does not already
  fail on — including one whose exception is recorded in `deny.toml` but whose
  reasoning has stopped being true.

## What is not

- **Anything that needs the operator's own privileges.** The configuration file
  and the model files are trusted input; the person who can edit them can
  already run the binary.
- **The absence of authentication.** It is deliberate, documented in §18, and
  the answer is a reverse proxy.
- **Denial of service by asking for expensive work.** A large `max_iterations`,
  a large lab batch, or a saturated write queue are capacity questions with
  documented backpressure ([§17](docs/architecture/17-reliability.md)), not
  vulnerabilities.
- **The LLM saying something unhelpful.** It cannot choose, validate, or
  influence a move — `CommentaryContext` has no move in it
  ([§2.3](docs/architecture/02-scope-and-constraints.md)). A finding that it
  *can* is a very different report, and a serious one.

## Release integrity

Every release tarball ships with a `.sha256` beside it, and the checksums are
verified in CI before the release is published. Every `uses:` in
[.github/workflows/](.github/workflows/) names a commit rather than a tag, so an
action's author cannot change what runs here by repointing a tag.
