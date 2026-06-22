# Plan Reviewer Prompt

```text
Subagent:
  description: "Review plan document"
  prompt: |
    Review plan: <PLAN_FILE_PATH>
    Spec: <SPEC_FILE_PATH>

    Approve unless a real implementation problem exists.
    Check:
    - placeholders/TODO/incomplete tasks
    - spec requirements covered, no major scope creep
    - task boundaries clear
    - steps actionable with exact files/code/commands
    - engineer could follow without guessing

    Output:
    ## Plan Review
    **Status:** Approved | Issues Found
    **Issues:** task/step, issue, why it blocks implementation
    **Recommendations:** advisory only
```
