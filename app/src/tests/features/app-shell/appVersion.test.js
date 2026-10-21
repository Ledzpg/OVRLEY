/** The version the app shows comes from the package version, so a local build never shows a placeholder. */

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cwd } from 'node:process'
import { describe, expect, test } from 'vitest'

describe('app version', () => {
  test('is defined at build time and matches package.json', () => {
    const { version } = JSON.parse(readFileSync(resolve(cwd(), 'package.json'), 'utf8'))

    expect(import.meta.env.VITE_OVRLEY_VERSION).toBe(version)
    expect(version).toMatch(/^\d+\.\d+\.\d+$/)
  })

  test('is the same version Tauri packages', () => {
    const { version } = JSON.parse(readFileSync(resolve(cwd(), '..', 'src-tauri', 'tauri.conf.json'), 'utf8'))

    expect(import.meta.env.VITE_OVRLEY_VERSION).toBe(version)
  })
})
