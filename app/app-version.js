/**
 * Version shown in the app. A release workflow can pass its tag in VITE_OVRLEY_VERSION; otherwise it is the package
 * version, which scripts/set-release-version.mjs keeps in step with the Tauri configuration.
 */

import path from 'path'
import process from 'process'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'

const packagePath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), 'package.json')

export const appVersion = process.env.VITE_OVRLEY_VERSION?.trim() || JSON.parse(readFileSync(packagePath, 'utf8')).version

/** Build-time constant that replaces `import.meta.env.VITE_OVRLEY_VERSION` in the app, for Vite and Vitest alike. */
export const appVersionDefine = { 'import.meta.env.VITE_OVRLEY_VERSION': JSON.stringify(appVersion) }
