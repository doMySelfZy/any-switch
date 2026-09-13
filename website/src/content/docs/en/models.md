---
title: Models
description: Fetch models from a site, type model ids, and probe availability.
order: 5
---

The model list belongs to the site, not a global catalog. Changing sites changes the list.

## Fetch

On the site detail, “Fetch models” calls the site’s models endpoint and stores the result locally. That only updates AnySwitch until you apply in Apply Center.

## Manual add

If the upstream list is incomplete or missing, use “Add manually” and type the model id the upstream actually accepts.

## Test

“Test” probes selected models, serially or in parallel. It needs a usable API key. A pass only means this route responded now, not a permanent guarantee.

## Cleanup

You can clear the whole list or delete selected rows. That only changes the local list; you can fetch again.

## Primary model

The site’s primary model is the default pick in Apply Center. Claude Code, Codex, and Pi still need their own confirm-and-apply step.
