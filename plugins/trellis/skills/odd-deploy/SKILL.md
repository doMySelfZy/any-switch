---
name: "odd-deploy"
version: "1.0.1"
description: "Trellis deploy command. Verify all outcomes are confirmed, then deploy the current phase to production."
---

# /trellis-deploy

You are executing the Trellis `*deploy` command.

Execute this flow:

1. Read `.odd/state.json` and confirm:
   - all outcomes in the current phase are verified
   - the git working tree is clean
   - the domain expert confirms they are ready to deploy
2. If any outcome is unverified, display which ones remain and route to `*status`.
3. If all outcomes are verified, run `vercel --prod`.
4. After deployment, display the production URL, update deployment fields in `.odd/state.json`, and store the deployment record in odd-flow memory.
