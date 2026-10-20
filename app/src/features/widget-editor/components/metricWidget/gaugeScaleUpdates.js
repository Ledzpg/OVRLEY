/** Pure update builders for the gauge fixed-scale and redline controls. */

export const DEFAULT_REDLINE_COLOR = '#ff3b30'

const REDLINE_DEFAULT_POSITION = 0.85

function parseNumber(raw) {
  if (raw === '' || raw == null) return null
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

function isInside(value, min, max) {
  return value > min && value < max
}

function defaultRedlineInside(min, max) {
  return min + (max - min) * REDLINE_DEFAULT_POSITION
}

/**
 * Builds the display-variant patch for one change to the scale / redline controls.
 *
 * Invalid input (non-numeric, max <= min, a redline outside a fixed scale) yields an empty patch so the
 * config can never reach the renderer in a state it would reject. When a scale change leaves the
 * redline outside the new range, the redline moves to 85% of the range.
 *
 * @param {object} data - Current display-variant data.
 * @param {{ min: number, max: number, redline: number }} defaults - Suggested values for the metric.
 * @param {object} change - One of `fixedScale`, `scaleMin`, `scaleMax`, `redline`, `redlineValue`.
 * @returns {object} Patch to merge into the display variant.
 */
export function buildGaugeScaleUpdate(data, defaults, change) {
  const fixed = data.scale_min != null && data.scale_max != null
  const range = fixed ? { min: data.scale_min, max: data.scale_max } : null

  if (Object.hasOwn(change, 'fixedScale')) {
    if (!change.fixedScale) return { scale_min: undefined, scale_max: undefined }
    const patch = { scale_min: defaults.min, scale_max: defaults.max }
    if (data.redline_value != null && !isInside(data.redline_value, defaults.min, defaults.max)) patch.redline_value = defaults.redline
    return patch
  }

  if (Object.hasOwn(change, 'scaleMin') || Object.hasOwn(change, 'scaleMax')) {
    if (!range) return {}
    const min = Object.hasOwn(change, 'scaleMin') ? parseNumber(change.scaleMin) : range.min
    const max = Object.hasOwn(change, 'scaleMax') ? parseNumber(change.scaleMax) : range.max
    if (min === null || max === null || max <= min) return {}
    const patch = { scale_min: min, scale_max: max }
    if (data.redline_value != null && !isInside(data.redline_value, min, max)) patch.redline_value = defaultRedlineInside(min, max)
    return patch
  }

  if (Object.hasOwn(change, 'redline')) {
    if (!change.redline) return { redline_value: undefined }
    const value = range
      ? isInside(defaults.redline, range.min, range.max)
        ? defaults.redline
        : defaultRedlineInside(range.min, range.max)
      : defaults.redline
    return { redline_value: value, redline_color: data.redline_color ?? DEFAULT_REDLINE_COLOR }
  }

  if (Object.hasOwn(change, 'redlineValue')) {
    const value = parseNumber(change.redlineValue)
    if (value === null || (range && !isInside(value, range.min, range.max))) return {}
    return { redline_value: value }
  }

  return {}
}
