/** Pure update builders for the raster tile map behind the route widget. */

import { COURSE_MAP_KEYS } from '@/lib/template/template-constants'

export const MAP_MIN_OPACITY = 5
export const MAP_MAX_ZOOM_OFFSET = 2

/**
 * Tile providers offered by the editor: only ones that serve tiles without an API key. Each requires showing its
 * attribution and has its own usage terms. Providers that need a key (Thunderforest, MapTiler, Stadia, Mapbox...)
 * go through the custom URL.
 */
export const MAP_PRESETS = Object.freeze([
  {
    id: 'osm',
    tileUrl: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
    attribution: '© OpenStreetMap contributors',
  },
  {
    id: 'opentopomap',
    tileUrl: 'https://tile.opentopomap.org/{z}/{x}/{y}.png',
    attribution: '© OpenStreetMap contributors, SRTM | © OpenTopoMap (CC-BY-SA)',
  },
  {
    id: 'esri-light-gray',
    tileUrl: 'https://server.arcgisonline.com/ArcGIS/rest/services/Canvas/World_Light_Gray_Base/MapServer/tile/{z}/{y}/{x}',
    attribution: 'Esri, HERE, Garmin, © OpenStreetMap contributors',
  },
])

export const CUSTOM_MAP_PRESET = 'custom'

const DEFAULT_PRESET = MAP_PRESETS[0]

function parseNumber(raw) {
  if (raw === '' || raw == null) return null
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

/**
 * Reports whether a tile URL template is one the renderer accepts.
 *
 * @param {string} url - Candidate template.
 * @returns {boolean} True for an https:// or file:// URL with {z}, {x} and {y}.
 */
export function isValidTileUrl(url) {
  return /^(https|file):\/\//.test(url) && ['{z}', '{x}', '{y}'].every((placeholder) => url.includes(placeholder))
}

/**
 * Finds the preset a widget uses.
 *
 * @param {object} data - Route widget data.
 * @returns {string} Preset id, or `custom` for any other tile URL.
 */
export function getMapPresetId(data) {
  return MAP_PRESETS.find((preset) => preset.tileUrl === data.map_tile_url)?.id ?? CUSTOM_MAP_PRESET
}

/**
 * Builds the widget-data patch for one change to the map controls.
 *
 * Invalid input (bad URL, empty credit, out-of-range numbers) yields an empty patch so the config never
 * reaches the renderer in a state it would reject. Every map field is set together and cleared together.
 *
 * @param {object} data - Current route widget data.
 * @param {object} change - One of `enabled` (boolean), `preset` (id), `tileUrl`, `attribution`, `opacity`, `zoomOffset`.
 * @returns {object} Patch to merge into the widget data.
 */
export function buildMapUpdate(data, change) {
  if (Object.hasOwn(change, 'enabled')) {
    if (!change.enabled) return Object.fromEntries(COURSE_MAP_KEYS.map((field) => [field, undefined]))
    return {
      map_tile_url: DEFAULT_PRESET.tileUrl,
      map_attribution: DEFAULT_PRESET.attribution,
      map_opacity: 100,
      map_zoom_offset: 0,
    }
  }
  if (data.map_tile_url == null) return {}

  if (Object.hasOwn(change, 'preset')) {
    const preset = MAP_PRESETS.find((candidate) => candidate.id === change.preset)
    return preset ? { map_tile_url: preset.tileUrl, map_attribution: preset.attribution } : {}
  }
  if (Object.hasOwn(change, 'tileUrl')) {
    const url = String(change.tileUrl).trim()
    return isValidTileUrl(url) ? { map_tile_url: url } : {}
  }
  if (Object.hasOwn(change, 'attribution')) {
    const text = String(change.attribution).trim()
    return text ? { map_attribution: text } : {}
  }
  if (Object.hasOwn(change, 'opacity')) {
    const opacity = parseNumber(change.opacity)
    // 1 would read as 100% in the renderer's percent/fraction handling, so the floor is above it.
    return opacity !== null && opacity >= MAP_MIN_OPACITY && opacity <= 100 ? { map_opacity: Math.round(opacity) } : {}
  }
  if (Object.hasOwn(change, 'zoomOffset')) {
    const offset = parseNumber(change.zoomOffset)
    return offset !== null && Number.isInteger(offset) && Math.abs(offset) <= MAP_MAX_ZOOM_OFFSET ? { map_zoom_offset: offset } : {}
  }
  return {}
}
