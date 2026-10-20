import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cwd } from 'node:process'
import { describe, expect, test } from 'vitest'
import { createBatchRenderTemplate } from '@/features/render-video/utils/renderConfig'
import { normalizeTemplateFilePayload } from '@/features/template-manager/utils/templateSnapshot'

// Vitest runs from app/ (see agents.md), so the bundled templates live one level up.
const TEMPLATES_DIR = resolve(cwd(), '..', 'templates')
const NAMES = ['cluster-tacometro-velocimetro', 'cluster-barras-verticais', 'painel-temperaturas', 'mapa-trajeto']

function loadTemplate(name) {
  return JSON.parse(readFileSync(resolve(TEMPLATES_DIR, `${name}.json`), 'utf8'))
}

describe.each(NAMES)('bundled template %s', (name) => {
  test('loads through the same normalization as the app', () => {
    const state = normalizeTemplateFilePayload(loadTemplate(name))
    expect(state.name).toBe(name)
    expect(state.config.scene).toMatchObject({ width: 1920, height: 1080 })
  })

  test('produces a complete backend config with unique widget ids', () => {
    const state = normalizeTemplateFilePayload(loadTemplate(name))
    const effective = createBatchRenderTemplate(state.config, state.settings.globalDefaults)
    const ids = [...effective.backdrops, ...effective.labels, ...effective.values].map((widget) => widget.id)
    expect(new Set(ids).size).toBe(ids.length)
    expect(effective.values.length).toBeGreaterThan(0)
    for (const widget of effective.values) {
      expect(widget.font).toBeTruthy()
      expect(widget.color).toBeTruthy()
    }
  })
})

describe('cluster template contents', () => {
  const effective = (name) => {
    const state = normalizeTemplateFilePayload(loadTemplate(name))
    return createBatchRenderTemplate(state.config, state.settings.globalDefaults)
  }

  test('the tachometer cluster has a dial with ticks, a redline, speed and a gear indicator', () => {
    const { values, backdrops } = effective('cluster-tacometro-velocimetro')
    const dial = values.find((widget) => widget.display_type === 'arc')
    expect(dial).toMatchObject({
      value: 'rpm',
      scale_min: 0,
      scale_max: 11000,
      redline_value: 10000,
      redline_fill_all: true,
      redline_blink_hz: 5,
      tick_major_interval: 1000,
    })
    expect(dial.fill_zones.map((zone) => zone.start)).toEqual([0, 7000, 8000])
    expect(dial.fill_zones[0]).toHaveProperty('end_color')
    // The warning band blends into the redline color, meeting it at the redline.
    expect(dial.fill_zones[2]).toMatchObject({ color: '#ffd400', end_color: dial.redline_color })
    expect(dial).toMatchObject({ needle_length: 40, needle_width: 10 })
    expect(values.some((widget) => widget.value === 'speed')).toBe(true)
    const gear = values.find((widget) => widget.value === 'gear_position')
    expect(gear).toMatchObject({ shadow_strength: 0, shadow_distance: 0 })
    expect(backdrops.some((widget) => widget.fill_color === '#ffd400')).toBe(true)
  })

  test('the map template puts the route on a raster map with its credit', () => {
    const state = normalizeTemplateFilePayload(loadTemplate('mapa-trajeto'))
    const effective = createBatchRenderTemplate(state.config, state.settings.globalDefaults)
    const route = effective.plots.find((plot) => plot.value === 'course')
    expect(route).toMatchObject({ map_zoom_offset: 0, map_opacity: 100, marker_variant: 'halo' })
    expect(route.map_tile_url).toMatch(/^https:\/\/.*\{z\}.*\{x\}.*\{y\}/)
    expect(route.map_attribution).toContain('OpenStreetMap')
  })

  test('the vertical-bars cluster uses vertical linear gauges with redlines', () => {
    const bars = effective('cluster-barras-verticais').values.filter((widget) => widget.display_type === 'linear')
    expect(bars.map((widget) => widget.value)).toEqual(['rpm', 'coolant_temperature'])
    for (const bar of bars) {
      expect(bar).toMatchObject({ orientation: 'vertical' })
      expect(bar.redline_value).toBeGreaterThan(bar.scale_min)
      expect(bar.redline_value).toBeLessThan(bar.scale_max)
    }
  })

  test('the temperature panel shows coolant and intake air dials in Celsius', () => {
    const dials = effective('painel-temperaturas').values.filter((widget) => widget.display_type === 'arc')
    expect(dials.map((widget) => widget.value)).toEqual(['coolant_temperature', 'intake_air_temperature'])
    expect(dials.every((widget) => widget.display_unit === 'celsius' && widget.tick_major_interval === 20)).toBe(true)
  })
})
