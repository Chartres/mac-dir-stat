# Deploy the landing page → macdirstat.dravec.org

Single static `index.html` + `screenshot.png` + `favicon.svg`. No build step.
Host: **Cloudflare Pages** (same setup as markdusk).

## One-time

```bash
npm i -g wrangler
wrangler login          # opens a browser — only you can do this (auth)
```

## Deploy

```bash
cd ~/my-git/mac-dir-stat
wrangler pages deploy site --project-name mac-dir-stat --commit-dirty=true
```

First run creates the `mac-dir-stat` Pages project and returns a `*.pages.dev`
URL. Re-run the same command to ship updates.

## Custom domain macdirstat.dravec.org

`dravec.org` must be a zone in this Cloudflare account. Then:
Pages → `mac-dir-stat` → Custom domains → Set up a custom domain →
`macdirstat.dravec.org`. Cloudflare adds the CNAME automatically.

## Notes

- The Download button points at
  `https://github.com/Chartres/mac-dir-stat/releases/latest` — resolves once a
  release is published (v0.5.1 is).
- `og:image` is absolute (`https://macdirstat.dravec.org/screenshot.png`) so
  link unfurls work. Update it if you pick a different domain.
- The existing GitHub Pages page (`chartres.github.io/mac-dir-stat`, the
  `gh-pages` branch) can stay as a fallback or be pointed here — your call.
