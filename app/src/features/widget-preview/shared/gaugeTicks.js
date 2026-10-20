/**
 * Dial ticks and numerals for arc and corner gauge previews. Mirrors the Rust renderer: ticks sit on the same
 * cap-aware angles as the fill (or the plain sweep for segmented bars) and marks past the redline use its color.
 */

import { getArcPoint } from '../widgets/arc-gauge/trackPath'
import { getArcFractionAngle } from './gaugeRedline'

/** Returns true when the gauge has dial ticks turned on. */
export function hasGaugeTicks(data) {
  return data?.tick_major_interval != null
}

/**
 * Lists every tick between `min` and `max`, with majors anchored at multiples of the interval.
 * @param {object} data - Gauge data with the tick fields.
 * @param {number} min - Scale minimum.
 * @param {number} max - Scale maximum.
 * @returns {Array<{ value: number, major: boolean }>} Tick marks in ascending order.
 */
export function getGaugeTickMarks(data, min, max) {
  const subdivisions = data.tick_minor_count + 1
  const step = data.tick_major_interval / subdivisions
  const first = Math.ceil(min / step - 1e-6)
  const last = Math.floor(max / step + 1e-6)
  const marks = []
  for (let index = first; index <= last; index += 1) {
    const value = index * step
    if (value < min - step * 1e-6 || value > max + step * 1e-6) continue
    marks.push({ value, major: ((index % subdivisions) + subdivisions) % subdivisions === 0 })
  }
  return marks
}

/** Formats a scaled tick value with a fixed number of decimals, never showing "-0". */
export function formatTickNumeral(value, decimals) {
  const text = value.toFixed(decimals)
  return Number(text) === 0 ? text.replace('-', '') : text
}

/**
 * Builds the tick lines and numerals of an arc-shaped gauge.
 *
 * @param {object} params
 * @param {object} params.data - Gauge data with the tick fields.
 * @param {object} params.trackGeometry - `{ centerX, centerY, radius, startAngle, sweepAngle, trackThickness }`.
 * @param {{ min: number, max: number }} params.range - Resolved scale.
 * @param {{ fraction: number, color: string }|null} params.redline - Resolved redline, if any.
 * @param {boolean} params.segmented - Whether the track is drawn as bars.
 * @param {Function} params.measureText - `(text, fontSize) => { boundsLeft, boundsRight, ascent, descent }`.
 * @returns {{ lines: object[], labels: object[] }} SVG-ready tick lines and numeral placements.
 */
export function getArcTickLayout({ data, trackGeometry, range, redline, segmented, measureText }) {
  const border = data.track_border_thickness
  const outer = trackGeometry.trackThickness + border * 2
  const side = data.tick_position === 'outside' ? 1 : -1
  const edge = trackGeometry.radius + side * (outer * 0.5 + data.tick_offset)
  const direction = Math.sign(trackGeometry.sweepAngle) || 1

  // The fill is measured on the border-inset track, exactly as the Rust renderer builds its reveal spec.
  const angleOffset =
    border > 0 && trackGeometry.radius > 0 ? Math.min(((border / trackGeometry.radius) * 180) / Math.PI, Math.abs(trackGeometry.sweepAngle) / 2) : 0
  const innerCorner = Math.max(0, data.track_corner_radius - border)
  const spec = {
    ...trackGeometry,
    startAngle: trackGeometry.startAngle + direction * angleOffset,
    sweepAngle: trackGeometry.sweepAngle - direction * angleOffset * 2,
    trackThickness: Math.max(0, trackGeometry.trackThickness - border * 2),
    startCornerRadius: innerCorner,
    endCornerRadius: data.track_fill_flat ? 0 : innerCorner,
  }

  const lines = []
  const labels = []
  for (const mark of getGaugeTickMarks(data, range.min, range.max)) {
    const fraction = (mark.value - range.min) / (range.max - range.min)
    const angle = segmented ? trackGeometry.startAngle + trackGeometry.sweepAngle * fraction : getArcFractionAngle(spec, fraction)
    const inRedline = redline != null && fraction >= redline.fraction
    const length = mark.major ? data.tick_major_length : data.tick_minor_length
    const from = getArcPoint(trackGeometry.centerX, trackGeometry.centerY, edge, angle)
    const to = getArcPoint(trackGeometry.centerX, trackGeometry.centerY, edge + side * length, angle)
    lines.push({ x1: from.x, y1: from.y, x2: to.x, y2: to.y, color: inRedline ? redline.color : data.tick_color, major: mark.major })

    if (!mark.major || !data.tick_show_labels) continue
    const text = formatTickNumeral(mark.value / data.tick_label_divisor, data.tick_label_decimals)
    const measurement = measureText(text, data.tick_label_font_size)
    const radians = (angle * Math.PI) / 180
    const halfExtent = Math.max(
      (measurement.boundsRight - measurement.boundsLeft) * 0.5 * Math.abs(Math.cos(radians)),
      data.tick_label_font_size * 0.35 * Math.abs(Math.sin(radians)),
    )
    const anchor = getArcPoint(trackGeometry.centerX, trackGeometry.centerY, edge + side * (length + data.tick_label_offset + halfExtent), angle)
    labels.push({
      text,
      x: anchor.x - (measurement.boundsLeft + measurement.boundsRight) * 0.5,
      baseline: anchor.y + (measurement.ascent - measurement.descent) * 0.5,
      color: inRedline ? redline.color : data.tick_label_color,
    })
  }
  return { lines, labels }
}
