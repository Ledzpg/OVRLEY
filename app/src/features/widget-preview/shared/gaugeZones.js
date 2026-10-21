/**
 * Fill color bands (solid or gradient) and the full redline fill for gauge previews. Mirrors the Rust renderer:
 * bands become color stops along the track, and a value at or past the redline (when it fills completely) paints the
 * whole track in the redline color, optionally blinking with the preview time.
 */

import { getArcFractionAngle, wedgePath } from './gaugeRedline'

const WEDGE_STEP_DEGREES = 3
const WEDGE_OVERLAP_DEGREES = 0.3

function parseHex(color) {
  const hex = color.replace('#', '')
  return [0, 2, 4].map((offset) => parseInt(hex.slice(offset, offset + 2), 16))
}

function toHex(channels) {
  return `#${channels.map((channel) => Math.round(channel).toString(16).padStart(2, '0')).join('')}`
}

/**
 * Color a band's gradient ends in: its own `end_color`, or with `end_follows_next` the next band's color (the redline
 * color for the last band).
 *
 * @param {Array<object>} zones - Bands in ascending order.
 * @param {number} index - Band whose end color is wanted.
 * @param {string|undefined} redlineColor - Redline color, if the gauge has a redline.
 * @returns {string|undefined} The color, or undefined for a solid band.
 */
export function resolveZoneEndColor(zones, index, redlineColor) {
  const zone = zones[index]
  if (zone.end_color != null) return zone.end_color
  if (zone.end_follows_next) return zones[index + 1]?.color ?? redlineColor ?? zone.color
  return undefined
}

/**
 * Resolves color bands into gradient stops along the track.
 * A solid band contributes its color at both ends; a gradient band blends to its `end_color` (or, with
 * `end_follows_next`, to the next band's color, or the redline color for the last band) at the start of the next band. The last band ends at the redline when one lies inside it, so a gradient there reaches its end color
 * exactly at the redline (the fill past the redline is the redline color anyway); without a redline it runs to the
 * scale end.
 *
 * @param {Array<{ start: number, color: string, end_color?: string, end_follows_next?: boolean }>|undefined} zones - Bands in
 *   ascending order.
 * @param {{ min: number, max: number }} range - Resolved gauge scale.
 * @param {number|null|undefined} [redlineValue] - Redline value, when the gauge has one.
 * @param {string|undefined} [redlineColor] - Redline color, where the last band's gradient ends when it follows the next band.
 * @returns {Array<{ fraction: number, color: string }>|null} Stops, or null when the gauge has no bands.
 */
export function getFillStops(zones, range, redlineValue, redlineColor) {
  if (!zones?.length || range.max <= range.min) return null
  const span = range.max - range.min
  const fractionOf = (value) => Math.min(1, Math.max(0, (value - range.min) / span))
  const stops = []
  zones.forEach((zone, index) => {
    const start = fractionOf(zone.start)
    const lastEnd = redlineValue != null && redlineValue > zone.start && redlineValue < range.max ? fractionOf(redlineValue) : 1
    const end = zones[index + 1] ? fractionOf(zones[index + 1].start) : lastEnd
    stops.push({ fraction: start, color: zone.color })
    stops.push({ fraction: end, color: resolveZoneEndColor(zones, index, redlineColor) ?? zone.color })
  })
  for (let index = 1; index < stops.length; index += 1) {
    if (stops[index].fraction <= stops[index - 1].fraction) stops[index].fraction = Math.min(1, stops[index - 1].fraction + 1e-5)
  }
  return stops
}

/**
 * Interpolated fill color at a position along the track.
 * @param {Array<{ fraction: number, color: string }>} stops - Stops from `getFillStops`.
 * @param {number} fraction - Position (0-1).
 * @returns {string} Hex color.
 */
export function getStopColorAt(stops, fraction) {
  if (fraction <= stops[0].fraction) return stops[0].color
  for (let index = 1; index < stops.length; index += 1) {
    const from = stops[index - 1]
    const to = stops[index]
    if (fraction > to.fraction) continue
    const t = Math.min(1, Math.max(0, (fraction - from.fraction) / Math.max(to.fraction - from.fraction, Number.EPSILON)))
    const a = parseHex(from.color)
    const b = parseHex(to.color)
    return toHex(a.map((channel, i) => channel + (b[i] - channel) * t))
  }
  return stops[stops.length - 1].color
}

/** Reports whether the value reached a redline that fills the whole track. */
export function isRedlineOverrun(data, value) {
  return data?.redline_fill_all === true && data.redline_value != null && value != null && value >= data.redline_value
}

/**
 * Visible phase of the blinking redline fill at a time.
 * @param {number|undefined} hz - Flashes per second; unset keeps the fill steady.
 * @param {number} seconds - Preview time.
 * @returns {boolean} True while the fill is shown.
 */
export function isRedlineBlinkOn(hz, seconds) {
  if (hz == null) return true
  return Math.floor(seconds * hz * 2) % 2 === 0
}

/**
 * Approximates an arc sweep gradient with thin wedges, each in the interpolated color at its angle.
 * Callers clip the wedges to the track shape.
 *
 * @param {object} geometry - Track geometry with cap radii (as `getArcFractionAngle` expects).
 * @param {Array<{ fraction: number, color: string }>} stops - Stops from `getFillStops`.
 * @returns {Array<{ d: string, color: string }>} Wedge paths and colors.
 */
export function getArcZoneWedges(geometry, stops) {
  const first = getArcFractionAngle(geometry, 0)
  const last = getArcFractionAngle(geometry, 1)
  const direction = last < first ? -1 : 1
  const count = Math.max(1, Math.ceil(Math.abs(last - first) / WEDGE_STEP_DEGREES))
  const reach = geometry.radius * 2 + geometry.trackThickness * 2 + 100
  const wedges = []
  for (let index = 0; index < count; index += 1) {
    const from = index / count
    const to = (index + 1) / count
    wedges.push({
      d: wedgePath(
        geometry.centerX,
        geometry.centerY,
        reach,
        getArcFractionAngle(geometry, from),
        getArcFractionAngle(geometry, to) + direction * WEDGE_OVERLAP_DEGREES,
      ),
      color: getStopColorAt(stops, (from + to) / 2),
    })
  }
  return wedges
}
