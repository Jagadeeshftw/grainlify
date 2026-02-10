# Grainlify documentation

This folder contains the [Docusaurus](https://docusaurus.io/) site for Grainlify docs. It is **docs-only** (no blog) and serves documentation at the site root.

## Prerequisites

- Node.js 20+

## Commands

- **Develop locally:** `npm start` — dev server at [http://localhost:3000](http://localhost:3000)
- **Build for production:** `npm run build` — output in `build/`
- **Preview production build:** `npm run serve` — serves `build/` locally

## Hosting

The build is static (HTML, CSS, JS). You can host it on any static host:

1. **Vercel / Netlify**  
   Point the project (or the `website` directory) at the repo. Set **Build command** to `npm run build` and **Output directory** to `website/build`. If the root is the repo root, set **Root directory** to `website`.

2. **GitHub Pages**  
   From the repo root, build from `website` and deploy the `website/build` folder (e.g. with `gh-pages` or GitHub Actions). In `docusaurus.config.js`, set `url` to your GitHub Pages URL and `baseUrl` to your repo path (e.g. `'/grainlify/'`).

3. **Any static host**  
   Run `cd website && npm run build` and upload the contents of `website/build/` to your server or CDN.

## Editing docs

- Edit or add Markdown files in `website/docs/`.
- Update `sidebars.js` to change sidebar order or add new pages.
- Change site title, logo, and footer in `docusaurus.config.js`.

Replace `static/img/logo.svg` with your own logo and update `url` in config for your production domain.

## Search

Search is disabled to avoid runtime errors with local search plugins in development. Use the sidebar and browser find (Ctrl+F / ⌘F) to navigate. To add search later, consider a plugin that supports docs-only mode and test with `npm run build` then `npm run serve`.
