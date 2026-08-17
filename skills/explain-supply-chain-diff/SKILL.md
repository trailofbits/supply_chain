---
name: explain-supply-chain-diff
description: Use to explain a `supply_chain` snapshot diff before it is blessed, classifying each publisher change as routine or as warranting escalation. Use when a `supply_chain` test fails with a snapshot difference, when reviewing a `BLESS=1` change, or when asked whether a crates.io owner change is benign.
metadata:
  version: "0.1.0"
  license: MIT OR Apache-2.0
  compatibility:
    requires:
      - shell access
      - jq
      - network access to the crates.io API
      - optionally, the gh command for corroborating GitHub accounts
---

# Explain a supply-chain snapshot diff

Blessing a snapshot is the act of accepting a supply-chain change. Explain the change before it is accepted.

Do not run `BLESS=1` or otherwise modify a snapshot unless the user explicitly requests it. Read-only inspection of the repository and the crates.io API is permitted.

## Scope

Analyze only the differences between the committed snapshot and the current report.

- Report publisher additions and removals, and crates entering or leaving the report.
- Distinguish user owners from team owners; they carry different meanings.
- Do not treat a difference as a finding merely because the snapshot changed. Most changes are routine.
- The report can include crates that are never built. Note when a changed crate is absent from the build graph, since such a change has no build-time impact.

## Enumerate the changes

Do not read the textual diff and summarize it. Diff hunks interleave crates and invite undercounting. Enumerate programmatically.

Set the snapshot path, then compare crate sets:

```
SNAPSHOT=tests/supply_chain.json
diff <(git show HEAD:"$SNAPSHOT" | jq -r '.crates_io_crates|keys[]') \
     <(jq -r '.crates_io_crates|keys[]' "$SNAPSHOT")
```

Then compare owners per crate:

```
for c in $(jq -r '.crates_io_crates|keys[]' "$SNAPSHOT"); do
  a=$(git show HEAD:"$SNAPSHOT" | jq -c --arg c "$c" '.crates_io_crates[$c]')
  b=$(jq -c --arg c "$c" '.crates_io_crates[$c]' "$SNAPSHOT")
  [ "$a" != "$b" ] && echo "CHANGED: $c"
done
```

For each changed crate, print owners before and after:

```
git show HEAD:"$SNAPSHOT" | jq -r --arg c "$c" '.crates_io_crates[$c][] | "  \(.kind) \(.login)"'
jq -r --arg c "$c" '.crates_io_crates[$c][] | "  \(.kind) \(.login)"' "$SNAPSHOT"
```

Repeat for every snapshot in the repository, including fixture snapshots. A crate may change in one and not another.

## Corroborate against crates.io

Confirm the new snapshot matches live ownership:

```
curl -s -H 'User-Agent: <project> supply-chain review (<contact>)' \
  "https://crates.io/api/v1/crates/$c/owners" \
  | jq -r '.users[]? | "  \(.kind // "user")  \(.login)"'
```

If live ownership disagrees with the snapshot, stop and investigate. The report may be stale, or the local toolchain may be producing something other than what crates.io reports. Resolve that before classifying anything.

## Investigate added owners

Establish identity and history before characterizing an addition.

1. Age and identity:

   ```
   curl -s -H 'User-Agent: ...' "https://crates.io/api/v1/users/<login>" | jq '.user'
   gh api users/<login> --jq '"type=\(.type) created=\(.created_at)"'
   ```

   Record `created_at` and the linked GitHub URL. A long-established account is weak evidence of legitimacy; a newly created one is strong evidence for escalation.

2. Breadth: query owners of sibling crates from the same organization or author. An account that already owns many related crates is behaving like an organizational account. An account appearing on exactly the crates that changed is not.

3. Correlate with a release. Query the crate's versions and compare publication dates with the ownership change. An owner added immediately before a new version is more interesting than one added in isolation.

## Investigate removed owners

- Note whether a team or an individual was removed, and whether comparable teams survive on sibling crates.
- Note whether the removal leaves a single owner.
- Removal alone transfers no publishing capability. Weigh it together with what was added.

## Classify

Classify a change as **routine** only when benign explanations are established, not merely plausible:

- every added owner is long-established and already owns sibling crates in the same ecosystem;
- the change is consistent with a reorganization visible across other crates of the same organization; and
- live crates.io ownership matches the snapshot.

Classify a change as **escalate** when any of the following holds:

- an added account was created recently, or owns no other crates in the ecosystem;
- all established owners were removed and replaced by one account with no independent corroboration;
- the ownership change coincides with a new release; or
- the snapshot and crates.io disagree.

Otherwise classify the change as **needs review** and state precisely what evidence is missing. When uncertain, choose **needs review**.

## Report

For each changed crate, report:

- owners before and after, marking each as user or team;
- classification and the evidence supporting it, including account creation dates and sibling-crate ownership;
- whether the crate is in the build graph, per `cargo tree -e normal`; and
- the recommended action: bless, or investigate further.

End with counts of crates examined, routine changes, changes needing review, and changes to escalate.

## Pitfalls

- **Determine an added account's age and breadth before calling a change suspicious.** "Several owners removed, one account added" is the shape of a takeover and also the shape of routine consolidation. The two are indistinguishable without querying the account. Do the query first; raising alarm and then retracting it costs the user's attention and trains them to discount the signal.
- **Enumerate changed crates with a loop, not by eye.** A diff that appears to concern one crate often concerns several.
- **A changed crate may not be built.** This crate's report is derived from `cargo metadata`, which over-approximates: weakly referenced optional dependencies and optional dependencies of dependencies appear even when disabled. Check `cargo tree -e normal` before describing impact.
- **Do not report a snapshot change as a finding because the tests failed.** Failing is the mechanism by which the change is surfaced, not evidence about it.

## Worked example

A `libc` ownership change surfaced simultaneously in two snapshots.

Enumeration found three changed crates in the root snapshot, not the one visible at the top of the diff:

| Crate | Before | After |
|---|---|---|
| `libc` | `huonw`, team `rust-lang:libs`, `joshtriplett`, `gnzlbg`, `JohnTitor`, `rust-lang-owner` | `rust-lang-owner` |
| `regex-syntax` | team `rust-lang:libs`, team `regex-owners`, `BurntSushi` | team `regex-owners`, `BurntSushi`, `rust-lang-owner` |
| `regex-automata` | `BurntSushi` | `BurntSushi`, `rust-lang-owner` |

The fixture snapshot changed only in `libc`, which reaches it as a dependency of a crate that is never built.

Investigation of the added account:

- `rust-lang-owner` was created 2019-07-19 on both crates.io and GitHub — six years established.
- It already owned `log` and `cc`, so it was not confined to the changed crates.
- `log` retained `huonw`, `sfackler`, `KodrAus`, and its team alongside it; `bitflags` had no `rust-lang-owner` at all. The change was a partial reorganization, not a sweep.
- Live crates.io ownership matched the new snapshot exactly for all three crates.

Classification: routine. The dramatic-looking reduction of `libc` from six owners to one was a long-established organizational account remaining after a team reorganization.

The error to avoid: this analysis was first reported as resembling a takeover, before the account's age and breadth had been checked. Both queries take seconds and would have settled it up front.
