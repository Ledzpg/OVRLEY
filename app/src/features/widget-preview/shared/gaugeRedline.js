/**
 * Fixed scale and redline helpers shared by the arc, corner and linear gauge previews.
 *
 * All values are in the metric's canonical unit (rpm, °C). The geometry mirrors the Rust renderer:
 * the redline sits at the same fraction of the track the fill reaches for that value.
 */

import { getArcPoint } from '../widgets/arc-gauge/trackPath'

const ZONE_REACH = 100000
const ARC_ZONE_STEP_DEGREES = 5
const ARC_MAX_ANGLE_DEGREES = 360

/** Returns the fixed `{ min, max }` scale configured on a gauge, or null when it follows the data. */
export function getFixedGaugeScale(data) {
  if (data?.scale_min == null || data?.scale_max == null) return null
  return { min: data.scale_min, max: data.scale_max }
}

/**
 * Places the configured redline on the resolved range.
 * @param {object} data - Gauge widget data.
 * @param {{ min: number, max: number }} range - Resolved gauge range.
 * @returns {{ fraction: number, color: string }|null} Redline position (0-1) and color, or null when none is set.
 */
export function getGaugeRedline(data, range) {
  if (data?.redline_value == null || range.max <= range.min) return null
  const fraction = Math.min(1, Math.max(0, (data.redline_value - range.min) / (range.max - range.min)))
  return { fraction, color: data.redline_color }
}

/** Returns the index of the first discrete bar that starts inside the redline zone. */
export function getRedlineBarStart(fraction, count) {
  return Math.min(count, Math.max(0, Math.ceil(fraction * count - 1e-9)))
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
 * @param {object} geometry - Track geometry plus cap radii and the redline `fraction`.
 * @returns {{ normal: string, redline: string }} SVG path data for each zone.
 */
export function getArcRedlineZonePaths({ fraction, ...geometry }) {
  const { first, last, direction, fullCircle } = getArcCapAwareSpan(geometry)
  const boundary = getArcFractionAngle(geometry, fraction)
  const margin = fullCircle ? 0 : 10
  const reach = geometry.radius * 2 + geometry.trackThickness * 2 + 100
  return {
    normal: wedgePath(geometry.centerX, geometry.centerY, reach, first - direction * margin, boundary),
    redline: wedgePath(geometry.centerX, geometry.centerY, reach, boundary, last + direction * margin),
  }
}

/**
 * Splits a linear track at the redline into clip rects for the normal and redline zones.
 * @param {{ x: number, y: number, width: number, height: number }} innerTrackRect - Track area the fill moves through.
 * @param {number} fraction - Redline position (0-1) along the fill direction.
 * @param {string} orientation - 'horizontal' fills left to right, 'vertical' bottom to top.
 * @returns {{ normal: object, redline: object }} Rect attributes for each zone.
 */
export function getLinearRedlineZoneRects(innerTrackRect, fraction, orientation) {
  const clamped = Math.min(1, Math.max(0, fraction))
  if (orientation === 'vertical') {
    const boundary = innerTrackRect.y + innerTrackRect.height * (1 - clamped)
    return {
      normal: { x: -ZONE_REACH, y: boundary, width: ZONE_REACH * 2, height: ZONE_REACH },
      redline: { x: -ZONE_REACH, y: boundary - ZONE_REACH, width: ZONE_REACH * 2, height: ZONE_REACH },
    }
  }
  const boundary = innerTrackRect.x + innerTrackRect.width * clamped
  return {
    normal: { x: boundary - ZONE_REACH, y: -ZONE_REACH, width: ZONE_REACH, height: ZONE_REACH * 2 },
    redline: { x: boundary, y: -ZONE_REACH, width: ZONE_REACH, height: ZONE_REACH * 2 },
  }
}
