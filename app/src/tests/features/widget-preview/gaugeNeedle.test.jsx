import { render, screen } from '@testing-library/react'
import { describe, expect, test } from 'vitest'
import { OverlayArcGaugeWidget } from '@/features/widget-preview/widgets/arc-gauge/ArcGaugePreview'
import { getArcNeedleLayout, hasGaugeNeedle } from '@/features/widget-preview/shared/gaugeNeedle'
import { buildNeedleUpdate, NEEDLE_DEFAULTS, NEEDLE_FIELDS } from '@/features/widget-editor/components/metricWidget/gaugeNeedleUpdates'
import { DISPLAY_VARIANT_KEYS, GAUGE_NEEDLE_KEYS } from '@/lib/template/template-constants'

const NEEDLE = {
  needle_length: 40,
  needle_width: 10,
  needle_offset: 5,
  needle_color: '#ff00ff',
  needle_border_thickness: 2,
  needle_fill_opacity: 0.5,
  needle_corner_radius: 2,
}

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
      track_thickness: 8,
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
      font_size: 40,
      color: '#ffffff',
      unit_color: '#ffffff',
      show_units: false,
      display_unit: 'rpm',
      decimals: 0,
      scale_min: 0,
      scale_max: 8000,
      ...NEEDLE,
      ...overrides,
    },
  }
}

const rpmActivity = (value) => ({ sample_elapsed_seconds: [0, 1], trim_end_seconds: 1, rpm: [value, value] })

const trackGeometry = { centerX: 180, centerY: 180, radius: 176, trackThickness: 8, startAngle: 135, sweepAngle: 270 }
const rotationOf = (layout) => Number(/rotate\(([-\d.]+)\)/.exec(layout.transform)[1])

describe('needle layout', () => {
  const data = { ...makeArc().data, track_border_thickness: 0, track_corner_radius: 0, track_fill_flat: true }

  test('turns with the value across the sweep', () => {
    const at = (fraction) => rotationOf(getArcNeedleLayout({ data, trackGeometry, segmented: false, fraction }))
    expect(at(0)).toBeCloseTo(135)
    expect(at(0.5)).toBeCloseTo(270)
    expect(at(1)).toBeCloseTo(405)
  })

  test('is centered on the track plus the offset and sized as configured', () => {
    const layout = getArcNeedleLayout({ data, trackGeometry, segmented: false, fraction: 0.5 })
    expect(layout.transform).toContain('translate(180 180)')
    expect(layout.x + layout.width / 2).toBeCloseTo(176 + 5)
    expect(layout).toMatchObject({ y: -5, width: 40, height: 10, cornerRadius: 2 })
  })

  test('clamps the corner radius to half of the needle', () => {
    const layout = getArcNeedleLayout({ data: { ...data, needle_corner_radius: 50 }, trackGeometry, segmented: false, fraction: 0 })
    expect(layout.cornerRadius).toBe(5)
  })

  test('bars use the plain sweep', () => {
    const rounded = { ...data, track_corner_radius: 20, track_fill_flat: false }
    const bars = rotationOf(getArcNeedleLayout({ data: rounded, trackGeometry, segmented: true, fraction: 0 }))
    const fill = rotationOf(getArcNeedleLayout({ data: rounded, trackGeometry, segmented: false, fraction: 0 }))
    expect(bars).toBeCloseTo(135)
    expect(fill).toBeLessThan(135)
  })

  test('is only present with needle_length', () => {
    expect(hasGaugeNeedle(makeArc().data)).toBe(true)
    expect(hasGaugeNeedle({ ...makeArc().data, needle_length: undefined })).toBe(false)
  })
})

describe('needle preview', () => {
  test('draws the needle with the configured style', () => {
    render(<OverlayArcGaugeWidget widget={makeArc()} activity={rpmActivity(4000)} previewSecond={0} globalScale={1} globalOpacity={1} />)
    const needle = screen.getByTestId('arc-gauge-needle')
    expect(needle.getAttribute('fill')).toBe('#ff00ff')
    expect(needle.getAttribute('fill-opacity')).toBe('0.5')
    expect(needle.getAttribute('stroke-width')).toBe('2')
    expect(needle.getAttribute('transform')).toMatch(/rotate\(27\d(\.\d+)?\)/)
  })

  test('draws no needle without needle_length', () => {
    render(
      <OverlayArcGaugeWidget
        widget={makeArc({ needle_length: undefined })}
        activity={rpmActivity(4000)}
        previewSecond={0}
        globalScale={1}
        globalOpacity={1}
      />,
    )
    expect(screen.queryByTestId('arc-gauge-needle')).toBeNull()
  })

  test('keeps marking the value while the redline fills the track', () => {
    const widget = makeArc({ redline_value: 6500, redline_color: '#ff0000', redline_fill_all: true })
    render(<OverlayArcGaugeWidget widget={widget} activity={rpmActivity(8000)} previewSecond={0} globalScale={1} globalOpacity={1} />)
    expect(screen.getByTestId('arc-gauge-needle').getAttribute('transform')).toMatch(/rotate\(40\d(\.\d+)?\)/)
  })
})

describe('needle controls', () => {
  test('enabling writes every field and disabling clears them all', () => {
    const on = buildNeedleUpdate({}, { enabled: true })
    expect(Object.keys(on).sort()).toEqual([...NEEDLE_FIELDS].sort())
    expect(on).toEqual(NEEDLE_DEFAULTS)
    const off = buildNeedleUpdate(NEEDLE, { enabled: false })
    expect(Object.values(off).every((value) => value === undefined)).toBe(true)
    expect(Object.keys(off)).toEqual(NEEDLE_FIELDS)
  })

  test('keeps values the user already set when re-enabling', () => {
    expect(buildNeedleUpdate({ needle_color: '#123456' }, { enabled: true }).needle_color).toBe('#123456')
  })

  test('ignores every change while the needle is off', () => {
    expect(buildNeedleUpdate({}, { field: 'needle_width', value: 5 })).toEqual({})
    expect(buildNeedleUpdate({}, { color: '#ffffff' })).toEqual({})
  })

  test('validates numeric fields', () => {
    expect(buildNeedleUpdate(NEEDLE, { field: 'needle_length', value: '55' })).toEqual({ needle_length: 55 })
    expect(buildNeedleUpdate(NEEDLE, { field: 'needle_length', value: 0 })).toEqual({})
    expect(buildNeedleUpdate(NEEDLE, { field: 'needle_width', value: -1 })).toEqual({})
    expect(buildNeedleUpdate(NEEDLE, { field: 'needle_offset', value: -12 })).toEqual({ needle_offset: -12 })
    expect(buildNeedleUpdate(NEEDLE, { field: 'needle_border_thickness', value: 0 })).toEqual({ needle_border_thickness: 0 })
    expect(buildNeedleUpdate(NEEDLE, { field: 'needle_corner_radius', value: -1 })).toEqual({})
    expect(buildNeedleUpdate(NEEDLE, { field: 'needle_length', value: '' })).toEqual({})
    expect(buildNeedleUpdate(NEEDLE, { field: 'unknown', value: 3 })).toEqual({})
  })

  test('the body opacity is entered in percent and stored as a fraction', () => {
    expect(buildNeedleUpdate(NEEDLE, { fillOpacity: 25 })).toEqual({ needle_fill_opacity: 0.25 })
    expect(buildNeedleUpdate(NEEDLE, { fillOpacity: 0 })).toEqual({ needle_fill_opacity: 0 })
    expect(buildNeedleUpdate(NEEDLE, { fillOpacity: 101 })).toEqual({})
  })

  test('only arc and corner variants carry needle fields', () => {
    for (const type of ['arc', 'corner']) for (const key of GAUGE_NEEDLE_KEYS) expect(DISPLAY_VARIANT_KEYS[type]).toContain(key)
    for (const key of GAUGE_NEEDLE_KEYS) expect(DISPLAY_VARIANT_KEYS.linear).not.toContain(key)
  })
})
