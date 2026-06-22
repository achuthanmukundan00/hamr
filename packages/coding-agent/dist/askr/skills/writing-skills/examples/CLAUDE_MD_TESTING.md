# Example: Testing Skill Discovery Instructions

Goal: find wording that makes agents check/use skills under pressure.

## Scenarios

Use real-choice prompts. Combine pressure with two options:

1. Time/confidence: production down, agent can fix fast vs check skill first.
2. Sunk cost: working solution already built vs read skill and maybe redo.
3. Authority/speed: human says "quick fix" vs check relevant skill.
4. Familiarity: common refactor agent knows vs check skill.

Prompt skeleton:

```text
IMPORTANT: real scenario. Choose and act.
Context: <pressure>
Options:
A) check/load relevant skill first (costs N min)
B) fastest direct action
What do you do?
```

## Variants

- NULL: no skill instructions.
- Soft: "consider checking".
- Directive: "before any task, check skills".
- Emphatic: "MUST check; failure if skipped".
- Process: explicit browse/search/read/follow steps.

## Protocol

1. Run NULL baseline; record choice/rationalization.
2. Run each variant on same scenario.
3. Add pressure; retest.
4. Meta-test failure: "Why did you skip? How should text change?"
5. Iterate on winner.

Success: checks unprompted, reads full skill, follows under pressure, cannot rationalize skip.

Expected: soft fails under pressure; directive partial; emphatic strongest; process may be longer but clear.
