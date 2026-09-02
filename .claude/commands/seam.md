---
description: Implement one of the todo!() seams against the section that owns it
argument-hint: "<seam — e.g. move generation, writer actor, tree search>"
---

Implement this seam: $ARGUMENTS

Follow the `implement-seam` skill. In short:

1. `grep -rn 'todo!' src` to locate it and read the § it names.
2. Read that architecture section **in full**, plus the §20 subsection that
   states the property the implementation must have.
3. Read the surrounding module — the types already encode half the decisions.
4. Implement, citing the § for every constant and non-obvious branch.
5. Write tests named for the property, not the function.
6. `just ci` until green.
7. Move the item from "Not yet implemented" to "Added" in `CHANGELOG.md`.

If the section does not settle a question the implementation needs answered,
stop and say so rather than inventing an answer — that is how the architecture
gets its next revision.
