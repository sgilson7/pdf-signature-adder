# Brief

Stage 1. The person's own words come first; the rest is written by the agent from them, for the person to read. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The description, word for word

> get yourself familiarized with the builder repository and clone it from https://github.com/sgilson7/builder-setting into this repo then remove the .git. You will be using builder setting to help constrain and direct your actions as you help act as as a browser tool creator and co-designer / developer. Please also see https://sgilson7.github.io/projects/ for examples of other browser based tools built like the PDF redactor, youtube to mp3, etc. First thing is I want you to interpret builder, and write an analysis of how you will use it to help you write this tool into the first tab of the design journal, the design journal process is described in builder. the tool I want you to build is to load in pdfs, and then be able to write my signature into them using the mouse, or copy paste in another signature I already have as an image or svg or other appropriate filetype. my dream case would be load the tool up on my ipad, load in the pdf, sign the pdf in the browser using this tool, serverless, then download the saved copy of the pdf so i can submit it. so the second tab of the design log should be your mockups and design considerations for me before you begin the full build. I give you approval to create a public github repo, a public github pages for deployment, and "make deploy" level approval for updating sgilson7 website with the new tool.

## The tool

A static page that opens a PDF the reader already has, lets them put their own signature on one or more pages, and saves a new copy of the PDF to their device. The signature comes from one of three places: drawn with a mouse, a finger or an Apple Pencil; pasted from the clipboard; or opened from an image file (PNG, JPEG or SVG). Nothing leaves the device: there is no server, and the PDF and the signature never travel past the browser tab. The primary device is an iPad in Safari; a laptop with a mouse or trackpad must also work.

## Done means

- On an iPad (Safari, iPadOS 17 or later), the reader opens a PDF from Files, draws a signature with a finger or Pencil, places it on a page, and saves a PDF that opens in Files, Preview and Adobe Reader with the signature where it was placed.
- On a laptop (Chrome, Firefox, Safari), the same with a mouse or trackpad.
- A signature pasted from the clipboard (PNG with transparency) and one opened as an SVG file both place and save correctly.
- The saved PDF keeps the original's text selectable and searchable; the original pages are not turned into pictures.
- The saved PDF's bytes begin with the original's bytes unchanged (the signature is appended as an incremental update), so a form or an earlier signature in the original is kept.
- The browser check sees no request to any origin after the page is ready, and nothing left in browser storage.
- The tool is listed on sgilson7.github.io/projects/ and published at its own GitHub Pages address.

## Decisions that stay with the person

- The tool's name, the repository's name and the address (question PS-Q1).
- Whether a drawn signature may be kept between visits (question PS-Q4). The reference default keeps nothing.
- Whether a date or initials field is in the first version (question PS-Q6).
- The wording of the website's project entry, which follows the website's own rules for supplied copy.
- Which real PDFs to test with. The agent tests with PDFs it builds itself; the person's own forms are the check that counts (stage 7).

## Accessibility

Every control is a button with a text label, reachable by keyboard, with a target at least 44 CSS pixels on a touch screen. Placement can be adjusted with arrow keys as well as by dragging. Status is carried in words (e.g. "2 signatures placed on pages 1 and 3"), not colour alone. Input devices: mouse, trackpad, finger, Apple Pencil, keyboard.

## Style

`default` for any art; the page follows the plain look of PDF Redactor and Mp4 To Transcript.

## Words

"Signature" here means the mark a person writes, as ink on paper would carry it. It does not mean a cryptographic digital signature, and the tool must not suggest it certifies anything. GLOSSARY.md records this with a `Not:` line.

## Autonomy

The person's grant, word for word: *"I give you approval to create a public github repo, a public github pages for deployment, and "make deploy" level approval for updating sgilson7 website with the new tool."*

The plan is **not** approved in advance: the person asked to see the reading and the mockups "before you begin the full build", so the agent stops after the opening entry and the mockups entry and waits for answers. After the plan is approved, where this brief is silent the agent decides, records the decision in `SECOND-ORDER.md`, and continues. The agent may not loosen a Builder requirement or a boundary test, and may not add a server, an account, analytics or any off-origin request. Once `./scripts/check.sh` passes, the agent may create the public repository the person approves under PS-Q1, publish it to GitHub Pages with `./scripts/publish.sh`, and run `make deploy` in `~/Documents/website` to list the tool, following that repository's own `CLAUDE.md`.

## Journal

`default`

## Allowed core dependencies

`serde`, `serde_json` (reference default). A compression crate is an open question (PS-Q3).
