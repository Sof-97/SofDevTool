# Text Diff renderer source

This directory owns the editable main JavaScript entry, exact npm lockfile and
build process used by the Rust application's embedded Text Diff renderer. The
optional Pierre edit bundle is not part of this application and is not built.

With Node.js 20 or newer and npm available, run:

```sh
npm ci
npm run build
npm run verify
```

`npm ci` installs the exact checked-in dependency graph. `npm run build` writes
the main bundle, its esbuild dependency graph and a matching notice inventory
to `../assets/`. `npm run verify` first checks the outer document appearance bridge, then rebuilds into a temporary directory and
compares all three files with the checked-in resources. The Rust renderer
embeds those files at compile time through `include_str!`; an installed app
does not run Node/npm, read the checkout, or fetch renderer resources.

The dependency manifest is derived from inputs that contribute bytes to
esbuild outputs, not every parsed module or package in the lockfile. The notice
generator includes license texts for those used packages, the retained entry
source attribution, and fails if a bundled package has no top-level license
file.

See [PROVENANCE.md](PROVENANCE.md) for the retained upstream source and bridge
adaptations.
