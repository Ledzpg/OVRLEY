import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

// Gathers what must be attached to the GitHub release: the signed Windows installer and the
// latest.json manifest the app downloads on startup. Run it after `build-installer.bat`.
//   node scripts/prepare-update-site.mjs [--notes "what changed"]

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const outputDir = resolve(rootDir, 'update-site')

const options = parseOptions(process.argv.slice(2))
const config = JSON.parse(await readFile(resolve(rootDir, 'src-tauri', 'tauri.conf.json'), 'utf8'))
const { productName, version } = config
const [endpoint] = config.plugins.updater.endpoints

const endpointMatch = /^(https:\/\/github\.com\/[^/]+\/[^/]+)\/releases\/latest\/download\/latest\.json$/.exec(endpoint)
if (!endpointMatch) {
  fail(`Updater endpoint must be https://github.com/<owner>/<repo>/releases/latest/download/latest.json, got ${endpoint}`)
}
const tag = `v${version}`
const downloadUrl = `${endpointMatch[1]}/releases/download/${tag}/`

const installerName = `${productName}_${version}_x64-setup.exe`
const installerPath = resolve(rootDir, 'src-tauri', 'target', 'release', 'bundle', 'nsis', installerName)
const signaturePath = `${installerPath}.sig`

const signature = await readOrFail(signaturePath, 'run build-installer.bat first, with the signing key available')
if (!signature.trim()) {
  fail(`Signature ${signaturePath} is empty`)
}

const manifest = {
  version,
  notes: options.notes,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': { signature: signature.trim(), url: `${downloadUrl}${installerName}` },
  },
}

await mkdir(outputDir, { recursive: true })
await copyFile(installerPath, resolve(outputDir, installerName))
await writeFile(resolve(outputDir, 'latest.json'), `${JSON.stringify(manifest, null, 2)}\n`)

console.log(`Version ${version} ready in ${outputDir}`)
console.log(`Publish with: gh release create ${tag} update-site/latest.json "update-site/${installerName}" --title ${tag}`)

async function readOrFail(path, hint) {
  try {
    return await readFile(path, 'utf8')
  } catch (error) {
    return fail(`Could not read ${path} (${hint}): ${error.message}`)
  }
}

function parseOptions(args) {
  const parsed = { notes: '' }
  for (let index = 0; index < args.length; index += 2) {
    if (args[index] !== '--notes' || args[index + 1] === undefined) {
      fail('Usage: node scripts/prepare-update-site.mjs [--notes "text"]')
    }
    parsed.notes = args[index + 1]
  }
  return parsed
}

function fail(message) {
  console.error(message)
  process.exit(1)
}
