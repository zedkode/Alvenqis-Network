# Install the Tauri-style Alvenqis documentation theme

Copy these files into the repository's existing `docs/` directory:

```text
index.html
.nojekyll
_404.md
_coverpage.md
_navbar.md
_sidebar.md
assets/docs.css
```

Keep all existing Markdown documents.

Then run:

```bash
git switch main
git pull origin main
git add docs
git commit -m "docs: align website with Alvenqis Control Center design"
git push origin main
```

GitHub Pages must use:

```text
Settings → Pages
Source: Deploy from a branch
Branch: main
Folder: /docs
```

Public URL:

```text
https://zedkode.github.io/Alvenqis-Network/
```

The logo is loaded from the canonical `Blockchain-prototype/alvenqis-desktop-v2/public/logo-mark.png`
asset through GitHub's raw content URL so the documentation uses the same mark
as Alvenqis Control Center.
