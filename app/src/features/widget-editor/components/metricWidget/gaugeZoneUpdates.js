/** Pure update builders for the gauge fill color bands and the full redline fill controls. */

import { resolveZoneEndColor } from '@/features/widget-preview/shared/gaugeZones'

export const MAX_FILL_ZONES = 12
export const MAX_REDLINE_BLINK_HZ = 20
export const DEFAULT_REDLINE_BLINK_HZ = 5

const DEFAULT_GRADIENT_START = '#2bd9ff'
const DEFAULT_GRADIENT_END = '#47ff6b'
const DEFAULT_ZONE_COLOR = '#ffd400'
const DEFAULT_REDLINE_COLOR = '#ff3b30'

function parseNumber(raw) {
  if (raw === '' || raw == null) return null
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

/**
 * Keeps color bands consistent with a new fixed scale: the first band starts at the scale minimum and bands
 * that are out of order or at/after the maximum are dropped.
 *
 * @param {Array<object>} zones - Current bands.
 * @param {number} min - New scale minimum.
 * @param {number} max - New scale maximum.
 * @returns {Array<object>} Bands valid for the scale (possibly empty).
 */
export function fitZonesToScale(zones, min, max) {
  const fitted = []
  zones.forEach((zone, index) => {
    const start = index === 0 ? min : zone.start
    if (start >= max || (fitted.length > 0 && start <= fitted[fitted.length - 1].start)) return
    fitted.push({ ...zone, start })
  })
  return fitted
}

/**
 * Builds the bands for a tachometer from its landmark speeds: a gradient up to the peak torque, a solid band up to
 * the peak power, then a warning band that blends into the redline color up to the redline. The gradients follow the
 * band after them (the redline color for the last one).
 *
 * @param {{ min: number, max: number }} range - Fixed scale.
 * @param {number} torque - Peak torque speed (rpm).
 * @param {number} power - Peak power speed (rpm).
 * @param {number|null} redline - Redline value, or null when there is none.
 * @param {string} [redlineColor] - Color the last band blends into.
 * @returns {Array<object>} Bands, or an empty array when the landmarks do not fit the scale in order.
 */
export function buildTorquePowerZones(range, torque, power, redline, redlineColor = DEFAULT_REDLINE_COLOR) {
  const ordered = range.min < torque && torque < power && power < range.max && (redline === null || power < redline)
  if (!ordered) return []
  return [
    { start: range.min, color: DEFAULT_GRADIENT_START, end_follows_next: true },
    { start: torque, color: DEFAULT_GRADIENT_END },
    // Without a redline there is no color to follow, so the warning band names its own end color.
    redline === null
      ? { start: power, color: DEFAULT_ZONE_COLOR, end_color: redlineColor }
      : { start: power, color: DEFAULT_ZONE_COLOR, end_follows_next: true },
  ]
}

function withZone(zones, index, patch) {
  return zones.map((zone, i) => (i === index ? { ...zone, ...patch } : zone))
}

/**
 * Builds the display-variant patch for one change to the color band controls.
 *
 * Bands need a fixed scale. Invalid input (non-numeric, out of order, outside the scale, too many bands) yields an
 * empty patch so the config never reaches the renderer in a state it would reject.
 *
 * @param {object} data - Current display-variant data.
 * @param {{ zonePreset?: { torque: number, power: number } }} defaults - Suggested values for the metric.
 * @param {object} change - One of `enabled`, `preset` (`torque`, `power`), `add`, `remove` (`index`), `start`, `color`,
 *   `gradient`, `followsNext`, `endColor` (each with `index` and `value`).
 * @returns {object} Patch to merge into the display variant.
 */
export function buildZonesUpdate(data, defaults, change) {
  if (data.scale_min == null || data.scale_max == null) return {}
  const range = { min: data.scale_min, max: data.scale_max }
  const zones = data.fill_zones ?? []

  if (Object.hasOwn(change, 'enabled')) {
    if (!change.enabled) return { fill_zones: undefined }
    const preset = defaults.zonePreset
    const generated = preset ? buildTorquePowerZones(range, preset.torque, preset.power, data.redline_value ?? null, data.redline_color) : []
    return {
      fill_zones: generated.length > 0 ? generated : [{ start: range.min, color: DEFAULT_GRADIENT_START, end_color: DEFAULT_GRADIENT_END }],
    }
  }

  if (Object.hasOwn(change, 'preset')) {
    const torque = parseNumber(change.preset.torque)
    const power = parseNumber(change.preset.power)
    if (torque === null || power === null) return {}
    const generated = buildTorquePowerZones(range, torque, power, data.redline_value ?? null, data.redline_color)
    return generated.length > 0 ? { fill_zones: generated } : {}
  }

  if (zones.length === 0) return {}

  if (Object.hasOwn(change, 'add')) {
    if (zones.length >= MAX_FILL_ZONES) return {}
    const last = zones[zones.length - 1]
    const start = Math.round(((last.start + range.max) / 2) * 100) / 100
    if (start <= last.start || start >= range.max) return {}
    return { fill_zones: [...zones, { start, color: resolveZoneEndColor(zones, zones.length - 1, data.redline_color) ?? last.color }] }
  }

  const index = change.index
  if (!Number.isInteger(index) || index < 0 || index >= zones.length) return {}

  if (Object.hasOwn(change, 'remove')) {
    if (zones.length === 1 || index === 0) return {}
    return { fill_zones: zones.filter((_, i) => i !== index) }
  }
  if (Object.hasOwn(change, 'start')) {
    const start = parseNumber(change.start)
    if (start === null || index === 0) return {}
    const previous = zones[index - 1].start
    const next = zones[index + 1]?.start ?? range.max
    if (start <= previous || start >= next) return {}
    return { fill_zones: withZone(zones, index, { start }) }
  }
  if (Object.hasOwn(change, 'color')) return { fill_zones: withZone(zones, index, { color: change.color }) }
  if (Object.hasOwn(change, 'gradient')) {
    // A new gradient follows the next band (the last band follows the redline color) until an end color is chosen.
    return { fill_zones: withZone(zones, index, { end_color: undefined, end_follows_next: change.gradient ? true : undefined }) }
  }
  if (Object.hasOwn(change, 'followsNext')) {
    if (change.followsNext) return { fill_zones: withZone(zones, index, { end_color: undefined, end_follows_next: true }) }
    // Switching it off keeps the color the gradient currently ends in, now as the band's own end color.
    const current = resolveZoneEndColor(zones, index, data.redline_color) ?? zones[index].color
    return { fill_zones: withZone(zones, index, { end_color: current, end_follows_next: undefined }) }
  }
  if (Object.hasOwn(change, 'endColor')) return { fill_zones: withZone(zones, index, { end_color: change.endColor, end_follows_next: undefined }) }
  return {}
}

/**
 * Builds the display-variant patch for the full redline fill and its blinking.
 *
 * @param {object} data - Current display-variant data.
 * @param {object} change - One of `fillAll` (boolean), `blink` (boolean) or `blinkHz` (number-like).
 * @returns {object} Patch to merge into the display variant.
 */
export function buildRedlineOverrunUpdate(data, change) {
  if (data.redline_value == null) return {}
  if (Object.hasOwn(change, 'fillAll')) {
    return change.fillAll ? { redline_fill_all: true } : { redline_fill_all: undefined, redline_blink_hz: undefined }
  }
  if (!data.redline_fill_all) return {}
  if (Object.hasOwn(change, 'blink')) return { redline_blink_hz: change.blink ? DEFAULT_REDLINE_BLINK_HZ : undefined }
  if (Object.hasOwn(change, 'blinkHz')) {
    const hz = parseNumber(change.blinkHz)
    return hz !== null && hz > 0 && hz <= MAX_REDLINE_BLINK_HZ ? { redline_blink_hz: hz } : {}
  }
  return {}
}
