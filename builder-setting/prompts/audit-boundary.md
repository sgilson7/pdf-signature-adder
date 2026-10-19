# Audit the boundary

Use before a deploy, or after any change that touched the shim or the page. It covers what the boundary tests cannot: a rule hidden in the page by means a source scan does not recognise.

```text
Review this repository and tell me whether any application rule has
leaked out of crates/core into crates/shim, web/ or data/.

A rule is anything a test of the tool's behaviour would need: a
comparison, a threshold, a lookup, a total, a choice of which message
to show, an ordering.

For each leak: name the file and line, say what rule it is, and show the
test in crates/core that should own it instead.

Separately, list any reader-visible string hard-coded in web/ or in
core source rather than held in data/, as a table: file, line, the
string, where it should live. Name the exceptions you think are
justified, with the reason.

Do not fix anything yet. List them first.
```
