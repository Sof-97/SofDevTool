import * as esbuild from 'esbuild';
import { createHash } from 'node:crypto';
import {
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const sourceRoot = dirname(fileURLToPath(import.meta.url));
const assetsRoot = resolve(sourceRoot, '../assets');
const packageManifest = JSON.parse(readFileSync(join(sourceRoot, 'package.json'), 'utf8'));
const lock = JSON.parse(readFileSync(join(sourceRoot, 'package-lock.json'), 'utf8'));

function packageForInput(input) {
  const parts = input.split(/[\\/]/);
  const moduleIndex = parts.lastIndexOf('node_modules');
  if (moduleIndex < 0 || moduleIndex + 1 >= parts.length) return null;

  const first = parts[moduleIndex + 1];
  const packageEnd = first.startsWith('@') ? moduleIndex + 3 : moduleIndex + 2;
  if (packageEnd > parts.length) return null;

  const packagePath = parts.slice(0, packageEnd).join(sep);
  const packageRoot = resolve(sourceRoot, packagePath);
  const metadata = JSON.parse(readFileSync(join(packageRoot, 'package.json'), 'utf8'));
  return { metadata, packagePath, packageRoot };
}

function licenseFor(packageRoot, metadata) {
  const candidates = readdirSync(packageRoot).filter((name) =>
    /^(license|licence|copying)(\.|$)/i.test(name),
  );
  if (candidates.length === 0) {
    throw new Error(`No top-level license text found for ${metadata.name}@${metadata.version}`);
  }
  const filename = candidates.sort((left, right) => left.localeCompare(right))[0];
  return { filename, text: readFileSync(join(packageRoot, filename), 'utf8').trim() };
}

function collectDependencies(metafile) {
  const used = new Map();
  const bundledInputs = new Set();
  for (const output of Object.values(metafile.outputs)) {
    for (const [input, detail] of Object.entries(output.inputs)) {
      if (detail.bytesInOutput > 0) bundledInputs.add(input);
    }
  }

  for (const input of [...bundledInputs].sort()) {
    const item = packageForInput(input);
    if (!item) continue;
    const lockKey = relative(sourceRoot, item.packageRoot).split(sep).join('/');
    const locked = lock.packages[lockKey];
    if (!locked || locked.version !== item.metadata.version) {
      throw new Error(
        `Bundled input ${input} resolves ${item.metadata.name}@${item.metadata.version}, ` +
          `which does not match the exact package lock entry ${lockKey}.`,
      );
    }

    let dependency = used.get(item.metadata.name);
    if (!dependency) {
      const license = licenseFor(item.packageRoot, item.metadata);
      dependency = {
        name: item.metadata.name,
        version: item.metadata.version,
        license: typeof item.metadata.license === 'string'
          ? item.metadata.license
          : JSON.stringify(item.metadata.license),
        licenseFile: license.filename,
        licenseText: license.text,
        files: [],
      };
      used.set(item.metadata.name, dependency);
    } else if (dependency.version !== item.metadata.version) {
      throw new Error(`The main bundle uses multiple versions of ${item.metadata.name}.`);
    }
    dependency.files.push(input);
  }
  return [...used.values()].sort((left, right) => left.name.localeCompare(right.name));
}

function renderDependencyManifest(dependencies) {
  return `${JSON.stringify(
    {
      upstream: {
        package: '@pierre/diffs',
        version: packageManifest.dependencies['@pierre/diffs'],
      },
      dependencies: dependencies.map(({ name, version, license, files }) => ({
        name,
        version,
        license,
        bundledFiles: files,
      })),
    },
    null,
    2,
  )}\n`;
}

function renderNotices(dependencies) {
  const rows = dependencies
    .map(({ name, version, license }) => `| \`${name}\` | \`${version}\` | ${license} |`)
    .join('\n');
  const licenseGroups = new Map();
  for (const dependency of dependencies) {
    const digest = createHash('sha256').update(dependency.licenseText).digest('hex');
    const group = licenseGroups.get(digest) || { packages: [], text: dependency.licenseText };
    group.packages.push(`${dependency.name} ${dependency.version}`);
    licenseGroups.set(digest, group);
  }
  const licenses = [...licenseGroups.values()]
    .sort((left, right) => left.packages[0].localeCompare(right.packages[0]))
    .map(
      ({ packages, text }) =>
        `### ${packages.join(', ')}\n\n\`\`\`text\n${text}\n\`\`\``,
    )
    .join('\n\n');
  const retainedSourceNotice = readFileSync(
    join(sourceRoot, 'RETAINED_SOURCE_LICENSE.md'),
    'utf8',
  ).trim();
  return [
    '# Third-party notices',
    '',
    'This inventory is generated from the packages present in the main Text Diff JavaScript bundle. It excludes packages used only by the optional editor bundle and unrelated Swift dependencies.',
    '',
    '| Package | Version | Declared license |',
    '| --- | --- | --- |',
    rows,
    '',
    '## License texts',
    '',
    licenses,
    '',
    retainedSourceNotice,
    '',
  ].join('\n');
}

async function build(outputRoot) {
  const outputFile = join(outputRoot, 'pierre-diffs-bundle.js');
  const result = await esbuild.build({
    absWorkingDir: sourceRoot,
    entryPoints: ['src/diff-entry.js'],
    outfile: outputFile,
    bundle: true,
    minify: true,
    format: 'iife',
    globalName: 'PierreDiffs',
    target: ['safari16'],
    legalComments: 'none',
    define: { 'process.env.NODE_ENV': '"production"' },
    loader: { '.css': 'text' },
    metafile: true,
  });
  const bundle = readFileSync(outputFile, 'utf8').replace(/[ \t]+$/gm, '');
  writeFileSync(outputFile, bundle);

  const dependencies = collectDependencies(result.metafile);
  if (!dependencies.some(({ name }) => name === '@pierre/diffs')) {
    throw new Error('The compiled bundle does not contain the pinned @pierre/diffs package.');
  }
  return {
    bundle,
    dependencies: renderDependencyManifest(dependencies),
    notices: renderNotices(dependencies),
  };
}

function writeAssets(outputRoot, generated) {
  writeFileSync(join(outputRoot, 'pierre-diffs-bundle.js'), generated.bundle);
  writeFileSync(join(outputRoot, 'bundle-dependencies.json'), generated.dependencies);
  writeFileSync(join(outputRoot, 'THIRD_PARTY_NOTICES.md'), generated.notices);
}

const checkOnly = process.argv.includes('--check');
if (checkOnly) {
  const temporaryRoot = mkdtempSync(join(tmpdir(), 'sofdevtool-text-diff-assets-'));
  try {
    const generated = await build(temporaryRoot);
    for (const [name, value] of [
      ['pierre-diffs-bundle.js', generated.bundle],
      ['bundle-dependencies.json', generated.dependencies],
      ['THIRD_PARTY_NOTICES.md', generated.notices],
    ]) {
      const checkedIn = readFileSync(join(assetsRoot, name), 'utf8');
      if (checkedIn !== value) {
        throw new Error(`${name} is stale; run npm run build in the Text Diff asset source.`);
      }
    }
    process.stdout.write('Text Diff bundle, dependency inventory and notices are reproducible.\n');
  } finally {
    rmSync(temporaryRoot, { recursive: true, force: true });
  }
} else {
  const generated = await build(assetsRoot);
  writeAssets(assetsRoot, generated);
  process.stdout.write(`Built local Text Diff bundle from ${packageManifest.name}.\n`);
}
