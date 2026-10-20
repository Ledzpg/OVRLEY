import { render, screen } from '@testing-library/react'
import { describe, expect, test } from 'vitest'
import { OverlayArcGaugeWidget } from '@/features/widget-preview/widgets/arc-gauge/ArcGaugePreview'
import { OverlayLinearGaugeWidget } from '@/features/widget-preview/widgets/linear-gauge/LinearGaugePreview'
import { getFillStops, getStopColorAt, isRedlineBlinkOn, isRedlineOverrun } from '@/features/widget-preview/shared/gaugeZones'
import {
  buildRedlineOverrunUpdate,
  buildTorquePowerZones,
  buildZonesUpdate,
  fitZonesToScale,
} from '@/features/widget-editor/components/metricWidget/gaugeZoneUpdates'
import { buildGaugeScaleUpdate } from '@/features/widget-editor/components/metricWidget/gaugeScaleUpdates'
import { getGaugeScaleDefaults } from '@/lib/widget/standard-metrics'
import { normalizeTemplateConfig } from '@/lib/template/template-normalization'

const ZONES = [
  { start: 0, color: '#0000ff', end_color: '#00ff00' },
  { start: 7000, color: '#00ff00' },
  { start: 8000, color: '#ffff00' },
]
const SCALE = {
  scale_min: 0,
  scale_max: 11000,
  redline_value: 10000,
  redline_color: '#ff0000',
  redline_fill_all: true,
  redline_blink_hz: 2,
  fill_zones: ZONES,
}
const rpmActivity = (value) => ({ sample_elapsed_seconds: [0, 1], trim_end_seconds: 1, rpm: [value, value] })

function makeArc(overrides = {}) {
  return {
    id: 'rpm-arc',
    type: 'rpm',
    category: 'values',
    data: {
      value: 'rpm',
      display_type: 'arc',
      opacity: 1,
      width: 360,
      height: 360,
      arc_angle: 270,
      track_thickness: 20,
      track_corner_radius: 0,
      track_border_thickness: 0,
      track_border_color: '#ffffff',
      track_empty_color: '#222222',
      track_empty_opacity: 0.5,
      track_filled_color: '#ffffff',
      track_filled_opacity: 1,
      track_fill_flat: true,
      track_fill_style: 'fill',
      inner_widget_offset_x: 0,
      inner_widget_offset_y: 0,
      show_min_max_labels: false,
      min_max_label_font: 'Arial.ttf',
      min_max_label_font_size: 12,
      min_max_label_color: '#ffffff',
      font: 'Arial.ttf',
      font_size: 30,
      color: '#ffffff',
      unit_color: '#ffffff',
      show_units: false,
      display_unit: 'rpm',
      decimals: 0,
      ...SCALE,
      ...overrides,
    },
  }
}

function makeLinear(overrides = {}) {
  return {
    id: 'rpm-linear',
    type: 'rpm',
    category: 'values',
    data: {
      value: 'rpm',
      display_type: 'linear',
      opacity: 1,
      width: 200,
      height: 40,
      orientation: 'horizontal',
      track_corner_radius: 0,
      track_border_thickness: 0,
      track_border_color: '#ffffff',
      track_empty_color: '#222222',
      track_empty_opacity: 0.5,
      track_filled_color: '#ffffff',
      track_filled_opacity: 1,
      track_fill_flat: true,
      track_fill_style: 'fill',
      show_min_max_labels: false,
      min_max_label_font: 'Arial.ttf',
      min_max_label_font_size: 12,
      min_max_label_color: '#ffffff',
      min_max_label_position: 'bottom',
      ...SCALE,
      ...overrides,
    },
  }
}

describe('fill stops', () => {
  const stops = getFillStops(ZONES, { min: 0, max: 11000 })

  test('bands become ordered stops along the track', () => {
    expect(stops.map((stop) => stop.color)).toEqual(['#0000ff', '#00ff00', '#00ff00', '#00ff00', '#ffff00', '#ffff00'])
    const fractions = stops.map((stop) => stop.fraction)
    expect(fractions[0]).toBe(0)
    expect(fractions[fractions.length - 1]).toBe(1)
    expect(fractions.every((fraction, index) => index === 0 || fraction > fractions[index - 1])).toBe(true)
  })

  test('the last band ends at the redline, so its gradient reaches the end color there', () => {
    const bands = [...ZONES.slice(0, 2), { start: 8000, color: '#ffff00', end_color: '#ff0000' }]
    const withRedline = getFillStops(bands, { min: 0, max: 11000 }, 10000)
    const last = withRedline[withRedline.length - 1]
    expect(last).toEqual({ fraction: 10000 / 11000, color: '#ff0000' })
    expect(getStopColorAt(withRedline, 9000 / 11000)).toBe('#ff8000')
    // Without a redline (or one outside the band) the last band runs to the scale end.
    expect(getFillStops(bands, { min: 0, max: 11000 })[5].fraction).toBe(1)
    expect(getFillStops(bands, { min: 0, max: 11000 }, 7500)[5].fraction).toBe(1)
  })

  test('colors interpolate inside a gradient band and stay solid elsewhere', () => {
    expect(getStopColorAt(stops, 0)).toBe('#0000ff')
    expect(getStopColorAt(stops, 3500 / 11000)).toBe('#008080')
    expect(getStopColorAt(stops, 7500 / 11000)).toBe('#00ff00')
    expect(getStopColorAt(stops, 9000 / 11000)).toBe('#ffff00')
  })

  test('no bands means no stops', () => {
    expect(getFillStops(undefined, { min: 0, max: 100 })).toBeNull()
    expect(getFillStops([], { min: 0, max: 100 })).toBeNull()
  })
})

describe('redline overrun and blinking', () => {
  test('overrun needs the option, a redline and a value at or past it', () => {
    expect(isRedlineOverrun(SCALE, 10000)).toBe(true)
    expect(isRedlineOverrun(SCALE, 9999)).toBe(false)
    expect(isRedlineOverrun({ ...SCALE, redline_fill_all: false }, 11000)).toBe(false)
    expect(isRedlineOverrun(SCALE, null)).toBe(false)
  })

  test('blink alternates with the time and is steady without a rate', () => {
    expect([0, 0.1, 0.3, 0.55].map((seconds) => isRedlineBlinkOn(2, seconds))).toEqual([true, true, false, true])
    expect(isRedlineBlinkOn(undefined, 0.3)).toBe(true)
  })
})

describe('arc preview with bands', () => {
  test('draws the band gradient as wedges and keeps the single color without bands', () => {
    render(<OverlayArcGaugeWidget widget={makeArc()} activity={rpmActivity(9000)} previewSecond={0} globalScale={1} />)
    const group = screen.getByTestId('arc-gauge-zone-fill')
    expect(group.querySelectorAll('path').length).toBeGreaterThan(20)
    const colors = new Set([...group.querySelectorAll('path')].map((path) => path.getAttribute('fill')))
    expect(colors).toContain('#ffff00')
    expect(screen.queryByTestId('arc-gauge-filled-track')).toBeNull()
  })

  test('without bands the fill keeps its single color', () => {
    render(<OverlayArcGaugeWidget widget={makeArc({ fill_zones: undefined })} activity={rpmActivity(9000)} previewSecond={0} globalScale={1} />)
    expect(screen.getByTestId('arc-gauge-filled-track')).toHaveAttribute('fill', '#ffffff')
    expect(screen.queryByTestId('arc-gauge-zone-fill')).toBeNull()
  })

  test('at the redline the whole arc fills in red, then hides in the blink-off phase', () => {
    const { unmount } = render(<OverlayArcGaugeWidget widget={makeArc()} activity={rpmActivity(10500)} previewSecond={0} globalScale={1} />)
    expect(screen.getByTestId('arc-gauge-overrun-fill')).toHaveAttribute('fill', '#ff0000')
    expect(screen.queryByTestId('arc-gauge-zone-fill')).toBeNull()
    unmount()
    render(<OverlayArcGaugeWidget widget={makeArc()} activity={{ ...rpmActivity(10500), trim_end_seconds: 1 }} previewSecond={0.3} globalScale={1} />)
    expect(screen.queryByTestId('arc-gauge-overrun-fill')).toBeNull()
    expect(screen.queryByTestId('arc-gauge-zone-fill')).toBeNull()
  })

  test('bars take their band color and turn red on overrun', () => {
    const bars = { track_fill_style: 'bars', bar_count: 11, bar_gap: 0 }
    const { unmount } = render(<OverlayArcGaugeWidget widget={makeArc(bars)} activity={rpmActivity(9000)} previewSecond={0} globalScale={1} />)
    const colors = screen.getAllByTestId(/arc-gauge-bar-(redline-)?filled/).map((node) => node.getAttribute('fill'))
    // Each bar is sampled at its center, so the first bar is almost, not exactly, the start color.
    expect(parseInt(colors[0].slice(5, 7), 16)).toBeGreaterThan(200)
    expect(parseInt(colors[0].slice(3, 5), 16)).toBeLessThan(40)
    expect(colors).toContain('#00ff00')
    expect(colors).toContain('#ffff00')
    unmount()
    render(<OverlayArcGaugeWidget widget={makeArc(bars)} activity={rpmActivity(10500)} previewSecond={0} globalScale={1} />)
    const overrunColors = screen.getAllByTestId('arc-gauge-bar-redline-filled').map((node) => node.getAttribute('fill'))
    expect(overrunColors).toHaveLength(11)
    expect(new Set(overrunColors)).toEqual(new Set(['#ff0000']))
  })
})

describe('linear preview with bands', () => {
  test('defines a gradient along the track and fills with it', () => {
    const { container } = render(<OverlayLinearGaugeWidget widget={makeLinear()} activity={rpmActivity(9000)} previewSecond={0} globalScale={1} />)
    const gradient = container.querySelector('linearGradient')
    expect(gradient).not.toBeNull()
    expect(gradient.querySelectorAll('stop')).toHaveLength(6)
    const fills = [...container.querySelectorAll('rect')].map((rect) => rect.getAttribute('fill'))
    expect(fills.some((fill) => fill === `url(#${gradient.getAttribute('id')})`)).toBe(true)
  })

  test('vertical gauges run the gradient bottom to top', () => {
    const { container } = render(
      <OverlayLinearGaugeWidget
        widget={makeLinear({ orientation: 'vertical', width: 40, height: 200 })}
        activity={rpmActivity(9000)}
        previewSecond={0}
        globalScale={1}
      />,
    )
    const gradient = container.querySelector('linearGradient')
    expect(Number(gradient.getAttribute('y1'))).toBeGreaterThan(Number(gradient.getAttribute('y2')))
    expect(gradient.getAttribute('x1')).toBe(gradient.getAttribute('x2'))
  })

  test('overrun fills the whole track in red and blinks off', () => {
    const { container, unmount } = render(
      <OverlayLinearGaugeWidget widget={makeLinear()} activity={rpmActivity(10500)} previewSecond={0} globalScale={1} />,
    )
    const full = [...container.querySelectorAll('rect')].find(
      (rect) => rect.getAttribute('fill') === '#ff0000' && rect.getAttribute('fill-opacity') === '1',
    )
    expect(full).toBeTruthy()
    expect(Number(full.getAttribute('width'))).toBeCloseTo(200)
    unmount()
    const hidden = render(<OverlayLinearGaugeWidget widget={makeLinear()} activity={rpmActivity(10500)} previewSecond={0.3} globalScale={1} />)
    const stillRed = [...hidden.container.querySelectorAll('rect')].filter(
      (rect) => rect.getAttribute('fill') === '#ff0000' && rect.getAttribute('fill-opacity') === '1',
    )
    expect(stillRed).toHaveLength(0)
  })
})

describe('band editor updates', () => {
  const defaults = getGaugeScaleDefaults('rpm')
  const base = { scale_min: 0, scale_max: 11000, redline_value: 10000, redline_color: '#ff0000' }

  test('rpm suggests the user-style peaks and builds three bands from them', () => {
    expect(defaults.zonePreset).toEqual({ torque: 7000, power: 8000 })
    const patch = buildZonesUpdate(base, defaults, { enabled: true })
    expect(patch.fill_zones).toEqual([
      { start: 0, color: '#2bd9ff', end_color: '#47ff6b' },
      { start: 7000, color: '#47ff6b' },
      { start: 8000, color: '#ffd400', end_color: '#ff0000' },
    ])
  })

  test('bands need a fixed scale', () => {
    expect(buildZonesUpdate({}, defaults, { enabled: true })).toEqual({})
  })

  test('the peaks must be ordered inside the scale and below the redline', () => {
    const range = { min: 0, max: 11000 }
    expect(buildTorquePowerZones(range, 7000, 8000, 10000)).toHaveLength(3)
    expect(buildTorquePowerZones(range, 8000, 7000, 10000)).toEqual([])
    expect(buildTorquePowerZones(range, 7000, 10500, 10000)).toEqual([])
    expect(buildTorquePowerZones(range, 0, 8000, null)).toEqual([])
    expect(buildZonesUpdate({ ...base, fill_zones: ZONES }, defaults, { preset: { torque: '6500', power: '8500' } }).fill_zones[1].start).toBe(6500)
    expect(buildZonesUpdate({ ...base, fill_zones: ZONES }, defaults, { preset: { torque: 'x', power: '8500' } })).toEqual({})
  })

  test('edits keep bands ordered and inside the scale', () => {
    const data = { ...base, fill_zones: ZONES }
    expect(buildZonesUpdate(data, defaults, { index: 1, start: '6500' }).fill_zones[1].start).toBe(6500)
    expect(buildZonesUpdate(data, defaults, { index: 1, start: '8000' })).toEqual({})
    expect(buildZonesUpdate(data, defaults, { index: 1, start: '-5' })).toEqual({})
    expect(buildZonesUpdate(data, defaults, { index: 0, start: '100' })).toEqual({})
    expect(buildZonesUpdate(data, defaults, { index: 2, start: '11000' })).toEqual({})
    expect(buildZonesUpdate(data, defaults, { index: 2, remove: true }).fill_zones).toHaveLength(2)
    expect(buildZonesUpdate(data, defaults, { index: 0, remove: true })).toEqual({})
  })

  test('a gradient on the last band blends into the redline color', () => {
    const data = {
      ...base,
      fill_zones: [
        { start: 0, color: '#00ff00' },
        { start: 8000, color: '#ffff00' },
      ],
    }
    expect(buildZonesUpdate(data, defaults, { index: 1, gradient: true }).fill_zones[1].end_color).toBe('#ff0000')
  })

  test('adds a band halfway to the scale end and toggles gradients', () => {
    const data = { ...base, fill_zones: ZONES }
    const added = buildZonesUpdate(data, defaults, { add: true }).fill_zones
    expect(added).toHaveLength(4)
    expect(added[3].start).toBe(9500)
    const gradient = buildZonesUpdate(data, defaults, { index: 1, gradient: true }).fill_zones[1]
    expect(gradient.end_color).toBe('#ffff00')
    expect(buildZonesUpdate(data, defaults, { index: 0, gradient: false }).fill_zones[0]).not.toHaveProperty('end_color', '#00ff00')
  })

  test('changing the scale refits the bands; turning it off clears them', () => {
    expect(fitZonesToScale(ZONES, 1000, 7500).map((zone) => zone.start)).toEqual([1000, 7000])
    const data = { ...base, fill_zones: ZONES }
    expect(buildGaugeScaleUpdate(data, defaults, { scaleMax: '7500' }).fill_zones.map((zone) => zone.start)).toEqual([0, 7000])
    expect(buildGaugeScaleUpdate(data, defaults, { fixedScale: false })).toHaveProperty('fill_zones', undefined)
  })

  test('the full redline fill and blinking follow the redline', () => {
    expect(buildRedlineOverrunUpdate({}, { fillAll: true })).toEqual({})
    expect(buildRedlineOverrunUpdate(base, { fillAll: true })).toEqual({ redline_fill_all: true })
    expect(buildRedlineOverrunUpdate(base, { blink: true })).toEqual({})
    const on = { ...base, redline_fill_all: true }
    expect(buildRedlineOverrunUpdate(on, { blink: true })).toEqual({ redline_blink_hz: 5 })
    expect(buildRedlineOverrunUpdate(on, { blinkHz: '8' })).toEqual({ redline_blink_hz: 8 })
    expect(buildRedlineOverrunUpdate(on, { blinkHz: '0' })).toEqual({})
    expect(buildRedlineOverrunUpdate(on, { blinkHz: '99' })).toEqual({})
    expect(buildRedlineOverrunUpdate(on, { fillAll: false })).toEqual({ redline_fill_all: undefined, redline_blink_hz: undefined })
  })

  test('turning the redline off clears its full-fill options', () => {
    const patch = buildGaugeScaleUpdate({ ...base, redline_fill_all: true, redline_blink_hz: 5 }, defaults, { redline: false })
    expect(patch).toEqual({ redline_value: undefined, redline_fill_all: undefined, redline_blink_hz: undefined })
  })
})

describe('band fields survive template normalization', () => {
  test('arc, corner and linear variants keep bands and the overrun options', () => {
    const scene = { width: 1920, height: 1080, fps: 30, updateRate: 1 }
    for (const displayType of ['arc', 'corner', 'linear']) {
      const config = {
        scene,
        backdrops: [],
        rasters: [],
        labels: [],
        plots: [],
        values: [
          {
            id: 'v1',
            value: 'rpm',
            x: 1,
            y: 1,
            display_type: displayType,
            display_variants: { [displayType]: { width: 100, height: 100, ...SCALE } },
          },
        ],
      }
      const variant = normalizeTemplateConfig(config, {}).values[0].display_variants[displayType]
      expect(variant.fill_zones).toEqual(ZONES)
      expect(variant).toMatchObject({ redline_fill_all: true, redline_blink_hz: 2 })
    }
  })
})
