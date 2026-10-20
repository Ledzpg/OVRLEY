import { render, screen } from '@testing-library/react'
import { describe, expect, test } from 'vitest'
import { OverlayArcGaugeWidget } from '@/features/widget-preview/widgets/arc-gauge/ArcGaugePreview'
import { formatTickNumeral, getGaugeTickMarks } from '@/features/widget-preview/shared/gaugeTicks'
import { buildGaugeTickUpdate, countGaugeTicks, TICK_STYLE_DEFAULTS } from '@/features/widget-editor/components/metricWidget/gaugeTickUpdates'
import { buildGaugeScaleUpdate } from '@/features/widget-editor/components/metricWidget/gaugeScaleUpdates'
import { getGaugeScaleDefaults } from '@/lib/widget/standard-metrics'
import { normalizeTemplateConfig } from '@/lib/template/template-normalization'

const TICKS = {
  tick_major_interval: 1000,
  tick_minor_count: 4,
  tick_position: 'inside',
  tick_color: '#ffffff',
  tick_major_length: 14,
  tick_minor_length: 7,
  tick_thickness: 2,
  tick_offset: 8,
  tick_show_labels: true,
  tick_label_divisor: 1000,
  tick_label_decimals: 0,
  tick_label_font_size: 22,
  tick_label_color: '#eeeeee',
  tick_label_offset: 6,
}
const SCALE = { scale_min: 0, scale_max: 8000, redline_value: 6500, redline_color: '#ff0000' }

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
      ...SCALE,
      ...TICKS,
      ...overrides,
    },
  }
}

const rpmActivity = (value) => ({ sample_elapsed_seconds: [0, 1], trim_end_seconds: 1, rpm: [value, value] })

describe('tick marks', () => {
  test('majors sit on multiples of the interval with minors between them', () => {
    const marks = getGaugeTickMarks(TICKS, 0, 8000)
    expect(marks).toHaveLength(41)
    expect(marks.filter((mark) => mark.major)).toHaveLength(9)
    expect(marks[1]).toMatchObject({ value: 200, major: false })
    expect(marks[5]).toMatchObject({ value: 1000, major: true })
  })

  test('a scale that does not start on a multiple only gets marks inside it', () => {
    const marks = getGaugeTickMarks({ tick_major_interval: 10, tick_minor_count: 1 }, 45, 80)
    expect(marks[0].value).toBe(45)
    expect(marks.every((mark) => mark.value >= 45 && mark.value <= 80)).toBe(true)
  })

  test('numerals never show negative zero', () => {
    expect(formatTickNumeral(-0.0001, 0)).toBe('0')
    expect(formatTickNumeral(2.5, 1)).toBe('2.5')
  })
})

describe('arc ticks preview', () => {
  test('draws every tick and one numeral per major tick', () => {
    render(<OverlayArcGaugeWidget widget={makeArc()} activity={rpmActivity(3000)} previewSecond={0} globalScale={1} />)
    expect(screen.getAllByTestId('arc-gauge-tick-major')).toHaveLength(9)
    expect(screen.getAllByTestId('arc-gauge-tick-minor')).toHaveLength(32)
    const numerals = [...screen.getByTestId('arc-gauge-ticks').querySelectorAll('text')].map((node) => node.textContent)
    expect(numerals).toEqual(['0', '1', '2', '3', '4', '5', '6', '7', '8'])
  })

  test('marks past the redline use the redline color', () => {
    render(<OverlayArcGaugeWidget widget={makeArc()} activity={rpmActivity(3000)} previewSecond={0} globalScale={1} />)
    const colors = [...screen.getByTestId('arc-gauge-ticks').querySelectorAll('line')].map((line) => line.getAttribute('stroke'))
    expect(colors.filter((color) => color === '#ff0000')).toHaveLength(8) // 6600..8000 every 200
    expect(colors.filter((color) => color === '#ffffff')).toHaveLength(33)
  })

  test('numerals can be hidden and ticks absent without the interval', () => {
    const { unmount } = render(
      <OverlayArcGaugeWidget widget={makeArc({ tick_show_labels: false })} activity={rpmActivity(3000)} previewSecond={0} globalScale={1} />,
    )
    expect(screen.getByTestId('arc-gauge-ticks').querySelectorAll('text')).toHaveLength(0)
    unmount()
    render(
      <OverlayArcGaugeWidget widget={makeArc({ tick_major_interval: undefined })} activity={rpmActivity(3000)} previewSecond={0} globalScale={1} />,
    )
    expect(screen.queryByTestId('arc-gauge-ticks')).toBeNull()
  })
})

describe('tick editor updates', () => {
  const defaults = getGaugeScaleDefaults('rpm')

  test('metric defaults suggest a tick layout for rpm and temperatures', () => {
    expect(defaults).toMatchObject({ tickInterval: 1000, tickMinor: 4, labelDivisor: 1000 })
    expect(getGaugeScaleDefaults('coolant_temperature')).toMatchObject({ tickInterval: 20, labelDivisor: 1 })
  })

  test('ticks need a fixed scale', () => {
    expect(buildGaugeTickUpdate({}, defaults, { enabled: true })).toEqual({})
  })

  test('enabling writes every required field', () => {
    const patch = buildGaugeTickUpdate({ scale_min: 0, scale_max: 8000 }, defaults, { enabled: true })
    expect(patch).toMatchObject({ tick_major_interval: 1000, tick_minor_count: 4, tick_label_divisor: 1000, ...TICK_STYLE_DEFAULTS })
  })

  test('keeps values the user already set when re-enabling', () => {
    const patch = buildGaugeTickUpdate({ scale_min: 0, scale_max: 8000, tick_color: '#123456' }, defaults, { enabled: true })
    expect(patch.tick_color).toBe('#123456')
  })

  test('rejects layouts that would exceed the tick limit', () => {
    const data = { scale_min: 0, scale_max: 8000, ...TICKS }
    expect(countGaugeTicks(0, 8000, 1000, 4)).toBe(41)
    expect(buildGaugeTickUpdate(data, defaults, { interval: '10' })).toEqual({})
    expect(buildGaugeTickUpdate(data, defaults, { minorCount: '12' })).toEqual({})
    expect(buildGaugeTickUpdate(data, defaults, { interval: '500' })).toEqual({ tick_major_interval: 500 })
  })

  test('validates numeric style fields', () => {
    const data = { scale_min: 0, scale_max: 8000, ...TICKS }
    expect(buildGaugeTickUpdate(data, defaults, { field: 'tick_thickness', value: '0' })).toEqual({})
    expect(buildGaugeTickUpdate(data, defaults, { field: 'tick_thickness', value: '3' })).toEqual({ tick_thickness: 3 })
    expect(buildGaugeTickUpdate(data, defaults, { field: 'tick_label_decimals', value: '1.5' })).toEqual({})
    expect(buildGaugeTickUpdate(data, defaults, { field: 'tick_offset', value: '-2' })).toEqual({})
    expect(buildGaugeTickUpdate(data, defaults, { position: 'sideways' })).toEqual({})
    expect(buildGaugeTickUpdate(data, defaults, { labelDivisor: '0' })).toEqual({})
  })

  test('turning the fixed scale off also turns ticks off', () => {
    const patch = buildGaugeScaleUpdate({ scale_min: 0, scale_max: 8000, ...TICKS }, defaults, { fixedScale: false })
    expect(patch).toHaveProperty('tick_major_interval', undefined)
  })
})

describe('tick fields survive template normalization', () => {
  const scene = { width: 1920, height: 1080, fps: 30, updateRate: 1 }
  const base = { backdrops: [], rasters: [], labels: [], plots: [] }
  const value = (displayType, extra) => ({
    id: 'v1',
    value: 'rpm',
    x: 1,
    y: 1,
    display_type: displayType,
    display_variants: { [displayType]: { width: 100, height: 100, ...extra } },
  })

  test.each(['arc', 'corner'])('%s keeps tick fields', (displayType) => {
    const normalized = normalizeTemplateConfig({ scene, ...base, values: [value(displayType, { ...SCALE, ...TICKS })] }, {})
    expect(normalized.values[0].display_variants[displayType]).toMatchObject(TICKS)
  })

  test('linear gauges do not carry tick fields', () => {
    const normalized = normalizeTemplateConfig({ scene, ...base, values: [value('linear', { ...SCALE, ...TICKS })] }, {})
    expect(normalized.values[0].display_variants.linear).not.toHaveProperty('tick_major_interval')
  })
})
