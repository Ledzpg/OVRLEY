/** Pure update builders for the arc gauge dial tick controls. */

/** Largest number of ticks (major plus minor) the renderer accepts. */
export const MAX_GAUGE_TICKS = 400
export const MAX_MINOR_TICKS_PER_MAJOR = 9

/** Style defaults written when ticks are turned on. */
export const TICK_STYLE_DEFAULTS = Object.freeze({
  tick_position: 'inside',
  tick_color: '#ffffff',
  tick_major_length: 14,
  tick_minor_length: 7,
  tick_thickness: 2,
  tick_offset: 8,
  tick_show_labels: true,
  tick_label_decimals: 0,
  tick_label_font_size: 22,
  tick_label_color: '#ffffff',
  tick_label_offset: 6,
})

/** Numeric tick fields editable as plain numbers, with their validity rules. */
const NUMERIC_FIELDS = {
  tick_major_length: { min: 0 },
  tick_minor_length: { min: 0 },
  tick_thickness: { min: 0, exclusiveMin: true },
  tick_offset: { min: 0 },
  tick_label_font_size: { min: 0, exclusiveMin: true },
  tick_label_offset: { min: 0 },
  tick_label_decimals: { min: 0, max: 4, integer: true },
}

function parseNumber(raw) {
  if (raw === '' || raw == null) return null
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

/**
 * Counts the ticks (major plus minor) a scale would draw.
 * @param {number} min - Scale minimum.
 * @param {number} max - Scale maximum.
 * @param {number} interval - Distance between major ticks.
 * @param {number} minorCount - Minor ticks between two majors.
 * @returns {number} Tick count.
 */
export function countGaugeTicks(min, max, interval, minorCount) {
  const step = interval / (minorCount + 1)
  return Math.floor(max / step + 1e-6) - Math.ceil(min / step - 1e-6) + 1
}

/** Reports whether a tick layout is drawable on a fixed scale. */
export function ticksFitScale(range, interval, minorCount) {
  return interval > 0 && countGaugeTicks(range.min, range.max, interval, minorCount) <= MAX_GAUGE_TICKS
}

/**
 * Builds the display-variant patch for one change to the tick controls.
 *
 * Ticks need a fixed scale; without one every change yields an empty patch. Invalid input (non-numeric,
 * out of range, too many ticks) also yields an empty patch so the config never reaches the renderer in a
 * state it would reject.
 *
 * @param {object} data - Current display-variant data.
 * @param {{ tickInterval: number, tickMinor: number, labelDivisor: number }} defaults - Suggested values for the metric.
 * @param {object} change - One of `enabled`, `interval`, `minorCount`, `position`, `showLabels`, `labelDivisor`,
 *   `field` + `value` (a numeric style field) or `color` + `value`.
 * @returns {object} Patch to merge into the display variant.
 */
export function buildGaugeTickUpdate(data, defaults, change) {
  if (data.scale_min == null || data.scale_max == null) return {}
  const range = { min: data.scale_min, max: data.scale_max }
  const interval = data.tick_major_interval ?? defaults.tickInterval
  const minorCount = data.tick_minor_count ?? defaults.tickMinor

  if (Object.hasOwn(change, 'enabled')) {
    if (!change.enabled) return { tick_major_interval: undefined }
    if (!ticksFitScale(range, interval, minorCount)) return {}
    const patch = {
      tick_major_interval: interval,
      tick_minor_count: minorCount,
      tick_label_divisor: data.tick_label_divisor ?? defaults.labelDivisor,
    }
    for (const [key, value] of Object.entries(TICK_STYLE_DEFAULTS)) patch[key] = data[key] ?? value
    return patch
  }

  if (data.tick_major_interval == null) return {}

  if (Object.hasOwn(change, 'interval')) {
    const next = parseNumber(change.interval)
    if (next === null || !ticksFitScale(range, next, minorCount)) return {}
    return { tick_major_interval: next }
  }
  if (Object.hasOwn(change, 'minorCount')) {
    const next = parseNumber(change.minorCount)
    if (next === null || !Number.isInteger(next) || next < 0 || next > MAX_MINOR_TICKS_PER_MAJOR || !ticksFitScale(range, interval, next)) return {}
    return { tick_minor_count: next }
  }
  if (Object.hasOwn(change, 'position')) {
    return change.position === 'inside' || change.position === 'outside' ? { tick_position: change.position } : {}
  }
  if (Object.hasOwn(change, 'showLabels')) return { tick_show_labels: change.showLabels === true }
  if (Object.hasOwn(change, 'labelDivisor')) {
    const next = parseNumber(change.labelDivisor)
    return next !== null && next > 0 ? { tick_label_divisor: next } : {}
  }
  if (Object.hasOwn(change, 'field')) {
    const rule = NUMERIC_FIELDS[change.field]
    const next = parseNumber(change.value)
    if (!rule || next === null) return {}
    if (rule.exclusiveMin ? next <= rule.min : next < rule.min) return {}
    if (rule.max != null && next > rule.max) return {}
    if (rule.integer && !Number.isInteger(next)) return {}
    return { [change.field]: next }
  }
  return {}
}
