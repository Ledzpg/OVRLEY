/** Pure update builders for the close-up route map controls. */

export const ZOOM_MIN = 8
export const ZOOM_MAX = 20
export const ZOOM_STEP = 0.5

function parseNumber(raw) {
  if (raw === '' || raw == null) return null
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

/** Numeric fields of the close-up map with their validity rules (the renderer rejects anything else). */
const NUMERIC_FIELDS = {
  zoom: { min: ZOOM_MIN, max: ZOOM_MAX },
  corner_radius: { min: 0 },
  completed_line_width: { min: 0, exclusiveMin: true },
  remaining_line_width: { min: 0, exclusiveMin: true },
  marker_size: { min: 0, exclusiveMin: true },
  marker_outline_width: { min: 0 },
}

/** Percent fields, stored as 0-100 like the other plot opacities. */
const PERCENT_FIELDS = ['completed_line_opacity', 'remaining_line_opacity', 'marker_opacity']

/**
 * Builds the widget-data patch for one change to a close-up map control.
 *
 * Invalid input (non-numeric, out of range) yields an empty patch so the config never reaches the renderer in
 * a state it would reject. Map style, credit, opacity and detail go through `buildMapUpdate`.
 *
 * @param {object} change - `field` + `value` for a numeric or percent field.
 * @returns {object} Patch to merge into the widget data.
 */
export function buildZoomUpdate(change) {
  const next = parseNumber(change.value)
  if (next === null) return {}
  if (PERCENT_FIELDS.includes(change.field)) return next >= 0 && next <= 100 ? { [change.field]: Math.round(next) } : {}
  const rule = NUMERIC_FIELDS[change.field]
  if (!rule) return {}
  if (rule.min != null && (rule.exclusiveMin ? next <= rule.min : next < rule.min)) return {}
  if (rule.max != null && next > rule.max) return {}
  return { [change.field]: next }
}
