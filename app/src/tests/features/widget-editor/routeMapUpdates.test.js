import { describe, expect, test } from 'vitest'
import { COURSE_MAP_KEYS, COURSE_PLOT_KEYS } from '@/lib/template/template-constants'
import { normalizeTemplateConfig } from '@/lib/template/template-normalization'
import { buildMapUpdate, CUSTOM_MAP_PRESET, getMapPresetId, isValidTileUrl, MAP_PRESETS } from '@/features/widget-editor/components/routeMapUpdates'

const withMap = {
  map_tile_url: MAP_PRESETS[0].tileUrl,
  map_attribution: MAP_PRESETS[0].attribution,
  map_opacity: 100,
  map_zoom_offset: 0,
}

describe('route map controls', () => {
  test('enabling sets every map field and disabling clears every one', () => {
    const enabled = buildMapUpdate({}, { enabled: true })
    expect(Object.keys(enabled).sort()).toEqual([...COURSE_MAP_KEYS].sort())
    expect(enabled.map_tile_url).toBe(MAP_PRESETS[0].tileUrl)

    const disabled = buildMapUpdate(withMap, { enabled: false })
    expect(Object.keys(disabled).sort()).toEqual([...COURSE_MAP_KEYS].sort())
    expect(Object.values(disabled).every((value) => value === undefined)).toBe(true)
  })

  test('every change is ignored while the map is off', () => {
    for (const change of [
      { preset: 'opentopomap' },
      { tileUrl: 'https://a/{z}/{x}/{y}.png' },
      { attribution: 'x' },
      { opacity: 50 },
      { zoomOffset: 1 },
    ]) {
      expect(buildMapUpdate({}, change)).toEqual({})
    }
  })

  test('a preset sets its URL and credit together', () => {
    expect(buildMapUpdate(withMap, { preset: 'opentopomap' })).toEqual({
      map_tile_url: 'https://tile.opentopomap.org/{z}/{x}/{y}.png',
      map_attribution: '© OpenStreetMap contributors, SRTM | © OpenTopoMap (CC-BY-SA)',
    })
    expect(buildMapUpdate(withMap, { preset: 'unknown' })).toEqual({})
  })

  test('tile URLs need https or file, and all three placeholders', () => {
    expect(isValidTileUrl('https://tiles.example/{z}/{x}/{y}.png')).toBe(true)
    expect(isValidTileUrl('file:///C:/tiles/{z}/{x}/{y}.png')).toBe(true)
    expect(isValidTileUrl('http://tiles.example/{z}/{x}/{y}.png')).toBe(false)
    expect(isValidTileUrl('https://tiles.example/{z}/{x}.png')).toBe(false)
    expect(buildMapUpdate(withMap, { tileUrl: '  https://t.example/{z}/{x}/{y}.png ' })).toEqual({
      map_tile_url: 'https://t.example/{z}/{x}/{y}.png',
    })
    expect(buildMapUpdate(withMap, { tileUrl: 'https://t.example/{z}.png' })).toEqual({})
  })

  test('the credit cannot be emptied', () => {
    expect(buildMapUpdate(withMap, { attribution: ' © Someone ' })).toEqual({ map_attribution: '© Someone' })
    expect(buildMapUpdate(withMap, { attribution: '   ' })).toEqual({})
  })

  test('opacity and detail stay inside what the renderer accepts', () => {
    expect(buildMapUpdate(withMap, { opacity: '60' })).toEqual({ map_opacity: 60 })
    // 1 would be read as 100% by the renderer's fraction/percent handling.
    expect(buildMapUpdate(withMap, { opacity: 1 })).toEqual({})
    expect(buildMapUpdate(withMap, { opacity: 101 })).toEqual({})
    expect(buildMapUpdate(withMap, { zoomOffset: -2 })).toEqual({ map_zoom_offset: -2 })
    expect(buildMapUpdate(withMap, { zoomOffset: 3 })).toEqual({})
    expect(buildMapUpdate(withMap, { zoomOffset: 0.5 })).toEqual({})
    expect(buildMapUpdate(withMap, { zoomOffset: '' })).toEqual({})
  })

  test('the provider select shows custom for any unknown URL', () => {
    expect(getMapPresetId(withMap)).toBe('osm')
    expect(getMapPresetId({ ...withMap, map_tile_url: 'https://own.example/{z}/{x}/{y}.png' })).toBe(CUSTOM_MAP_PRESET)
  })
})

describe('route map persistence', () => {
  test('the map keys are course plot keys', () => {
    for (const key of COURSE_MAP_KEYS) expect(COURSE_PLOT_KEYS).toContain(key)
  })

  test('a saved template keeps the map settings and omits them when there is no map', () => {
    const plot = { value: 'course', id: 'course-1', x: 0, y: 0, width: 300, height: 300, ...withMap }
    const config = { scene: {}, rasters: [], plots: [plot] }
    const kept = normalizeTemplateConfig(config, null).plots[0]
    for (const key of COURSE_MAP_KEYS) expect(kept[key]).toBe(withMap[key])

    const bare = normalizeTemplateConfig({ ...config, plots: [{ value: 'course', id: 'course-1', x: 0, y: 0, width: 300, height: 300 }] }, null)
      .plots[0]
    for (const key of COURSE_MAP_KEYS) expect(Object.hasOwn(bare, key)).toBe(false)
  })
})
