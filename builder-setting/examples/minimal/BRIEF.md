# Brief

Stage 1. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The request, word for word

> A counter or similarly small deterministic state machine is sufficient. It exists only to demonstrate a rule in the Rust core, a fast core test, the WebAssembly shim, a static page, a browser interaction, the browser-network check, and the complete Builder check command.

## The tool

A counter with Add, Remove and Reset. The count stays between 0 and a maximum read from `data/counter.json`, and moves by a step read from the same file.

## Done means

- `./scripts/check.sh` passes.
- Someone can read the whole example quickly.

## Decisions that stay with the person

None beyond the reference defaults.

## Autonomy

The plan is approved in advance. The agent may build and test; it may not push or deploy.

## Journal

`default`: `../minimal-journal/`, kept in the reference repository beside this example so its form check runs here.

## Allowed core dependencies

`serde`, `serde_json`.
