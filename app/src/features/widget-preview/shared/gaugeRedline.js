/**
 * Fixed scale and redline helpers shared by the arc, corner and linear gauge previews.
 *
 * All values are in the metric's canonical unit (rpm, °C). The geometry mirrors the Rust renderer:
 * the redline sits at the same fraction of the track the fill reaches for that value.
 */

import { getStandardMetricRedlineZone } from '@/lib/widget/standard-metrics'
import { getArcPoint } from '../widgets/arc-gauge/trackPath'

const ZONE_REACH = 100000
const ARC_ZONE_STEP_DEGREES = 5
const ARC_MAX_ANGLE_DEGREES = 360

/** Returns the fixed `{ min, max }` scale configured on a gauge, or null when it follows the data. */
export function getFixedGaugeScale(data) {
  if (data?.scale_min == null || data?.scale_max == null) return null
  return { min: data.scale_min, max: data.scale_max }
}

/** Reports whether the gauge's redline is a critical level, marking the values at or under it instead of past it. */
export function isCriticalLevel(data) {
  return getStandardMetricRedlineZone(data.value) === 'below'
}

/**
 * Places the configured redline on the resolved range.
 * @param {object} data - Gauge widget data.
 * @param {{ min: number, max: number }} range - Resolved gauge range.
 * @returns {{ fraction: number, color: string, below: boolean }|null} Redline position (0-1), color and side, or null when none is set.
 */
export function getGaugeRedline(data, range) {
  if (data?.redline_value == null || range.max <= range.min) return null
  const fraction = Math.min(1, Math.max(0, (data.redline_value - range.min) / (range.max - range.min)))
  return { fraction, color: data.redline_color, below: isCriticalLevel(data) }
}

/** Returns the index of the first discrete bar that starts inside the redline zone. */
export function getRedlineBarStart(fraction, count) {
  return Math.min(count, Math.max(0, Math.ceil(fraction * count - 1e-9)))
}

/** Reports whether a position along the track (0-1) lies inside the redline zone. */
export function isFractionInRedline(redline, fraction) {
  return redline.below ? fraction <= redline.fraction : fraction >= redline.fraction
}

/** Reports whether a continuous fill of `fill` (0-1) reaches into the redline zone. */
export function fillEntersRedline(redline, fill) {
  return redline.below ? fill > 0 : fill > redline.fraction
}

/** Reports whether the discrete bar at `index` lies inside the redline zone; `barStart` is the redline split index. */
export function isRedlineBar(redline, index) {
  return redline.below ? index < redline.barStart : index >= redline.barStart
}

/** Splits discrete bars into the ones outside and inside the redline zone. */
export function splitRedlineBars(items, redline) {
  const head = items.slice(0, redline.barStart)
  const tail = items.slice(redline.barStart)
  return redline.below ? { normal: tail, redline: head } : { normal: head, redline: tail }
}

function arcCapAngle(radius, cornerRadius) {
  return cornerRadius > 0 ? (Math.atan2(cornerRadius, radius) * 180) / Math.PI : 0
}

export function wedgePath(centerX, centerY, reach, from, to) {
  const steps = Math.max(1, Math.ceil(Math.abs(to - from) / ARC_ZONE_STEP_DEGREES))
  const commands = [`M ${centerX} ${centerY}`]
  for (let step = 0; step <= steps; step += 1) {
    const point = getArcPoint(centerX, centerY, reach, from + ((to - from) * step) / steps)
    commands.push(`L ${point.x} ${point.y}`)
  }
  commands.push('Z')
  return commands.join(' ')
}

/**
 * Returns the cap-aware span of an arc track: the screen-space angles where the rounded track starts and ends
 * (end caps included), its direction and whether it is a full circle. Mirrors the Rust fill reveal.
 */
function getArcCapAwareSpan({ radius, startAngle, sweepAngle, trackThickness, startCornerRadius, endCornerRadius }) {
  const magnitude = Math.min(Math.abs(sweepAngle), ARC_MAX_ANGLE_DEGREES)
  const direction = sweepAngle < 0 ? -1 : 1
  const fullCircle = magnitude >= ARC_MAX_ANGLE_DEGREES - 1e-3
  const halfThickness = trackThickness * 0.5
  const startCap = fullCircle ? 0 : arcCapAngle(radius, Math.min(Math.max(startCornerRadius, 0), halfThickness))
  const endCap = fullCircle ? 0 : arcCapAngle(radius, Math.min(Math.max(endCornerRadius, 0), halfThickness))
  const first = startAngle - direction * startCap
  const last = first + direction * (magnitude + startCap + endCap)
  return { first, last, direction, fullCircle }
}

/**
 * Screen-space angle where a fill of `fraction` ends, so ticks and the redline line up with the filled track.
 * @param {object} geometry - Track geometry plus cap radii.
 * @param {number} fraction - Position along the track (0-1).
 * @returns {number} Angle in degrees.
 */
export function getArcFractionAngle(geometry, fraction) {
  const { first, last } = getArcCapAwareSpan(geometry)
  return first + (last - first) * Math.min(1, Math.max(0, fraction))
}

/**
 * Splits an arc track at the redline into clip paths for the normal and redline zones.
 * The boundary uses the same cap-aware span as the arc fill reveal.
 * @param {object} geometry - Track geometry plus cap radii and the redline `fraction`; `below` marks the start side instead of the end side.
 * @returns {{ normal: string, redline: string }} SVG path data for each zone.
 */
export function getArcRedlineZonePaths({ fraction, below, ...geometry }) {
  const { first, last, direction, fullCircle } = getArcCapAwareSpan(geometry)
  const boundary = getArcFractionAngle(geometry, fraction)
  const margin = fullCircle ? 0 : 10
  const reach = geometry.radius * 2 + geometry.trackThickness * 2 + 100
  const before = wedgePath(geometry.centerX, geometry.centerY, reach, first - direction * margin, boundary)
  const after = wedgePath(geometry.centerX, geometry.centerY, reach, boundary, last + direction * margin)
  return below ? { normal: after, redline: before } : { normal: before, redline: after }
}

/**
 * Splits a linear track at the redline into clip rects for the normal and redline zones.
 * @param {{ x: number, y: number, width: number, height: number }} innerTrackRect - Track area the fill moves through.
 * @param {number} fraction - Redline position (0-1) along the fill direction.
 * @param {string} orientation - 'horizontal' fills left to right, 'vertical' bottom to top.
 * @param {boolean} below - True when the redline zone is the start side of the boundary (a critical level).
 * @returns {{ normal: object, redline: object }} Rect attributes for each zone.
 */
export function getLinearRedlineZoneRects(innerTrackRect, fraction, orientation, below) {
  const clamped = Math.min(1, Math.max(0, fraction))
  if (orientation === 'vertical') {
    const boundary = innerTrackRect.y + innerTrackRect.height * (1 - clamped)
    const before = { x: -ZONE_REACH, y: boundary, width: ZONE_REACH * 2, height: ZONE_REACH }
    const after = { x: -ZONE_REACH, y: boundary - ZONE_REACH, width: ZONE_REACH * 2, height: ZONE_REACH }
    return below ? { normal: after, redline: before } : { normal: before, redline: after }
  }
  const boundary = innerTrackRect.x + innerTrackRect.width * clamped
  const before = { x: boundary - ZONE_REACH, y: -ZONE_REACH, width: ZONE_REACH, height: ZONE_REACH * 2 }
  const after = { x: boundary, y: -ZONE_REACH, width: ZONE_REACH, height: ZONE_REACH * 2 }
  return below ? { normal: after, redline: before } : { normal: before, redline: after }
}
