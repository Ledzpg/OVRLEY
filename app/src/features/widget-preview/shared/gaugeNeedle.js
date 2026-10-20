/**
 * Dial needle for arc and corner gauge previews. Mirrors the Rust renderer: a rounded bar centered on the track
 * (plus the offset) and rotated to the angle where a fill of the current value ends.
 */

import { getArcValueAngle } from './gaugeTicks'

/** Returns true when the gauge has a needle turned on. */
export function hasGaugeNeedle(data) {
  return data?.needle_length != null
}

/**
 * Computes the SVG placement of the needle.
 *
 * @param {object} params
 * @param {object} params.data - Gauge data with the needle fields.
 * @param {object} params.trackGeometry - Track geometry of the gauge.
 * @param {boolean} params.segmented - Whether the track is drawn as bars.
 * @param {number} params.fraction - Current fill fraction (0-1), not the redline-overrun fill.
 * @returns {{ transform: string, x: number, y: number, width: number, height: number, cornerRadius: number,
 *   fill: { color: string, opacity: number, end: { color: string, opacity: number }|null } }} Rect, transform and body fill.
 *   A gradient body runs from the end toward the center (`x`) to the outer end (`x + width`) of the rect.
 */
export function getArcNeedleLayout({ data, trackGeometry, segmented, fraction }) {
  const angle = getArcValueAngle({ data, trackGeometry, segmented, fraction })
  const centerRadius = trackGeometry.radius + data.needle_offset
  return {
    transform: `translate(${trackGeometry.centerX} ${trackGeometry.centerY}) rotate(${angle})`,
    x: centerRadius - data.needle_length / 2,
    y: -data.needle_width / 2,
    width: data.needle_length,
    height: data.needle_width,
    cornerRadius: Math.min(data.needle_corner_radius, data.needle_width / 2, data.needle_length / 2),
    // The body color is optional: without it the border color fills the body.
    fill: {
      color: data.needle_fill_color ?? data.needle_color,
      opacity: data.needle_fill_opacity,
      end: data.needle_fill_end_color == null ? null : { color: data.needle_fill_end_color, opacity: data.needle_fill_end_opacity },
    },
  }
}
