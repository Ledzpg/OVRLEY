/** Pure update builders for the arc gauge needle controls. */

/** Values written when the needle is turned on. */
export const NEEDLE_DEFAULTS = Object.freeze({
  needle_length: 36,
  needle_width: 8,
  needle_offset: 0,
  needle_color: '#ffffff',
  needle_border_thickness: 2,
  needle_fill_opacity: 0.35,
  needle_corner_radius: 3,
})

export const NEEDLE_FIELDS = Object.keys(NEEDLE_DEFAULTS)

/** Numeric needle fields with their validity rules. The offset may be negative (inward). */
const NUMERIC_FIELDS = {
  needle_length: { min: 0, exclusiveMin: true },
  needle_width: { min: 0, exclusiveMin: true },
  needle_offset: {},
  needle_border_thickness: { min: 0 },
  needle_corner_radius: { min: 0 },
}

function parseNumber(raw) {
  if (raw === '' || raw == null) return null
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

/**
 * Builds the display-variant patch for one change to the needle controls.
 *
 * Invalid input (non-numeric, out of range) yields an empty patch so the config never reaches the renderer in a
 * state it would reject. Turning the needle on writes every field; turning it off clears them all.
 *
 * @param {object} data - Current display-variant data.
 * @param {object} change - One of `enabled` (boolean), `field` + `value` (a numeric field), `color` or
 *   `fillOpacity` (percent, 0-100).
 * @returns {object} Patch to merge into the display variant.
 */
export function buildNeedleUpdate(data, change) {
  if (Object.hasOwn(change, 'enabled')) {
    if (!change.enabled) return Object.fromEntries(NEEDLE_FIELDS.map((field) => [field, undefined]))
    return Object.fromEntries(NEEDLE_FIELDS.map((field) => [field, data[field] ?? NEEDLE_DEFAULTS[field]]))
  }
  if (data.needle_length == null) return {}

  if (Object.hasOwn(change, 'color')) return { needle_color: change.color }
  if (Object.hasOwn(change, 'fillOpacity')) {
    const percent = parseNumber(change.fillOpacity)
    return percent !== null && percent >= 0 && percent <= 100 ? { needle_fill_opacity: percent / 100 } : {}
  }
  if (Object.hasOwn(change, 'field')) {
    const rule = NUMERIC_FIELDS[change.field]
    const next = parseNumber(change.value)
    if (!rule || next === null) return {}
    if (rule.min != null && (rule.exclusiveMin ? next <= rule.min : next < rule.min)) return {}
    return { [change.field]: next }
  }
  return {}
}
