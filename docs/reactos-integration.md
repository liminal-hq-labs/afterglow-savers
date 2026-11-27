# ReactOS integration plan

How reactos-liminal would build and install the savers. This repository does not edit reactos-liminal; the plan is for its maintainers. The full plan is written once the `.scr` files have run in the guest; the fixed points so far:

- **The Afterglow recipes** (base, full and compact) install the four Afterglow savers and the Collection: `AGMAZE.SCR`, `AGCRACK.SCR`, `AGTWILIT.SCR`, `AGINK.SCR` and `AGCOLLCT.SCR`, into `ReactOS\system32`.
- **Quill Ghostwriter** (`AGQUILL.SCR`, listed as "Quill Ghostwriter") goes **only** in Quill's recipe, as an extra `[[files]]` entry into `ReactOS\system32`, and is never the selected saver (SAV-D22). It is not in any Afterglow recipe.
