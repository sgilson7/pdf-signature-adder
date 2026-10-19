# Start from a description

For a person who does not write code. Clone Builder, start a coding agent in the clone, and describe the tool in your own words. The agent turns the description into a brief, reads Builder and your description back to you in a design journal, builds the tool, and, if you allow it, publishes it. Later requests are more plain prose. Stages 1–9.

## What the person does

```bash
git clone https://github.com/sgilson7/builder-setting.git
cd builder-setting
claude
```

Then write what you want, in as much or as little detail as you have. Say what it is for, who uses it, what a good result looks like, and anything from your own field the tool has to get right. If you want it published, name the repository and the address, and say so. For example:

```text
I teach first-year chemistry. I want a page where students balance an
equation by dragging coefficients, and the page tells them which element
is still out of balance. Use the reactions in the attached list. Make a
public repository called balance-lab and publish it on my GitHub Pages.
You may publish once the checks pass.
```

The agent creates the project in a new folder next to the clone, so it may ask permission to write there. For later sessions, start the agent inside that project folder.

Everything after that is in plain prose too: "the hint should name the element, not the side", "add a timer", "the buttons are too small on a phone".

## What the agent does

The root `CLAUDE.md` sends the agent here when a person describes a tool. The agent follows these steps without further prompting.

```text
1. Make the project. Copy template/ to a sibling directory named after the
   tool (lowercase, hyphens), run git init there, and work only there. Do
   not edit the reference repository. Then read the project's CLAUDE.md
   (or AGENTS.md) in full: an agent started in the reference repository is
   not given it automatically, and its rules govern everything below.
   Start the design journal: python3 tools/journal/new_journal.py, which
   makes ../<name>-journal/ beside the project as its own repository.

2. Write BRIEF.md. Paste the description word for word under "The
   description, word for word". Then fill the rest of the brief from it:
   the tool, done means (each condition checkable), decisions that stay
   with the person, accessibility, style ("default" unless the description
   names a look), words, autonomy. Where the description gives a term from
   the person's field, put it in GLOSSARY.md with a plain definition and,
   where the field has near-synonyms that mean something else, a Not: line.
   Leave names of people and employers out of a public repository.
   Leave the Journal section at `default` unless the person said not to
   keep one. Then write the journal's opening entry (tools/journal/ENTRIES.md,
   kind reading): what Builder asks of you, what this project changes, what
   you measured, numbered rules with what holds each, the question prefix,
   where you stop, your questions. Add a brief entry: the description word
   for word and where each requirement lands.

3. Decide the autonomy paragraph from what the person said. If they asked
   for the tool to be published and named where, write that grant into
   the paragraph, word for word, and record it in DECISIONS.md. Without
   that, the agent does not publish.

4. Ask only what blocks the build: a fact from the person's field the
   agent cannot know, or content only they can supply. Number the
   questions with the journal's prefix (XX-Q1 ...) and put them in the
   opening entry. Every other decision gets a default, listed in a "for
   you" journal entry and recorded in SECOND-ORDER.md, and the agent
   continues. The person may answer by number; an answer that differs
   from a default becomes a TRIAGE.md row citing the question. Rules the
   answers settle for every later session go under "Rules agreed for this
   project" in CLAUDE.md and AGENTS.md.

5. Plan, measure, build with tests, keep the notebook, as CLAUDE.md says.
   The plan is approved in advance unless the person asked to read it, so
   the notebook is where the person will see what was decided for them.
   Write a row in SECOND-ORDER.md at the moment you make a choice the
   brief did not fix, find that an assumption was wrong, or find something
   only the person can do, and commit it with the change it describes.
   Number rows R1.1, R1.2 …; mark the person's rows with status person.
   Give each visible change, divergence from the plan and deploy a journal
   entry when it happens, with pictures of the running build, citing its
   rows. Never remove an entry.

6. Run ./scripts/check.sh. If the brief grants publishing: create the
   repository the person named, push, run ./scripts/publish.sh, and run
   the browser check against the live address. Give the person the
   address and the repository link.

7. Write HANDOFF.md, and tell the person in plain words what was built,
   what they should try by hand first, and what is still open: each
   SECOND-ORDER.md row with status person, by its number, so they can
   answer by number. Add a "ready for you" journal entry and give the
   person the journal, as a link if it is published. Tell them that later
   sessions start in the project directory (cd ../<name> && claude), where
   the project's own CLAUDE.md is loaded from the start.

8. For each later request: add a numbered row to TRIAGE.md with the
   person's words, make the change with its test, cite the row in the
   commit ("(T3)"), run the check, and republish if the brief allows it.
   Add a "your fixes" journal entry for the round: the row, what the
   person said, what changed, with pictures.
   A request that would break a Builder requirement is answered with the
   reason and the nearest option that keeps it.
```

## What the person still owns

The content and its correctness in their field, whether the tool does what they meant, anything to do with the people who will use it, and the decision to make it public. The agent's checks show that the code keeps Builder's rules and that the cases written into the tests hold. They cannot show that the cases are the right ones. The person's first use of the tool, with a case worked out by hand, is the check that can.
