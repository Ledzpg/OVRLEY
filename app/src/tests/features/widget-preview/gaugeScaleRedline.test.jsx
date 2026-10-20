import { render, screen } from '@testing-library/react'
import { describe, expect, test } from 'vitest'
import { OverlayLinearGaugeWidget } from '@/features/widget-preview/widgets/linear-gauge/LinearGaugePreview'
import { OverlayArcGaugeWidget } from '@/features/widget-preview/widgets/arc-gauge/ArcGaugePreview'
import { getArcGaugeLayout } from '@/features/widget-preview/widgets/arc-gauge/geometry'
import { getLinearGaugeLayout } from '@/features/widget-preview/widgets/linear-gauge/geometry'
import { getArcRedlineZonePaths, getLinearRedlineZoneRects, getRedlineBarStart } from '@/features/widget-preview/shared/gaugeRedline'
import { buildGaugeScaleUpdate } from '@/features/widget-editor/components/metricWidget/gaugeScaleUpdates'
import { getGaugeScaleDefaults } from '@/lib/widget/standard-metrics'

const SCALE = { scale_min: 0, scale_max: 11000, redline_value: 8800, redline_color: '#ff0000' }

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
      track_filled_color: '#40e0d0',
      track_filled_opacity: 1,
      track_fill_flat: false,
      track_fill_style: 'fill',
      show_min_max_labels: false,
      min_max_label_font: 'Arial.ttf',
      min_max_label_font_size: 12,
      min_max_label_color: '#ffffff',
      min_max_label_position: 'bottom',
      ...overrides,
    },
  }
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
      width: 160,
      height: 160,
      arc_angle: 180,
      track_thickness: 14,
      track_corner_radius: 0,
      track_border_thickness: 0,
      track_border_color: '#ffffff',
      track_empty_color: '#222222',
      track_empty_opacity: 0.5,
      track_filled_color: '#40e0d0',
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
      ...overrides,
    },
  }
}

const rpmActivity = (value) => ({ sample_elapsed_seconds: [0, 1], trim_end_seconds: 1, rpm: [value, value] })

describe('fixed scale layout', () => {
  test('linear layout ignores the observed range and places the redline', () => {
    const layout = getLinearGaugeLayout({ value: 5500, values: [3000, 4000], width: 200, height: 40, scale: SCALE })
    expect(layout).toMatchObject({ min: 0, max: 11000, fill: 0.5 })
    expect(layout.redline.fraction).toBeCloseTo(0.8)
    expect(layout.redline.color).toBe('#ff0000')
  })

  test('arc layout ignores the observed range and places the redline', () => {
    const layout = getArcGaugeLayout({ ...makeArc().data, ...SCALE }, 5500, [3000, 4000])
    expect(layout).toMatchObject({ min: 0, max: 11000, fill: 0.5 })
    expect(layout.redline.fraction).toBeCloseTo(0.8)
  })

  test('without scale fields the range follows the data and there is no redline', () => {
    const layout = getLinearGaugeLayout({ value: 2000, values: [0, 4000], width: 200, height: 40, scale: {} })
    expect(layout).toMatchObject({ min: 0, max: 4000, fill: 0.5, redline: null })
  })
})

describe('redline geometry', () => {
  test('redline bars start at the first bar whose start reaches the threshold', () => {
    expect(getRedlineBarStart(0.8, 10)).toBe(8)
    expect(getRedlineBarStart(0.85, 10)).toBe(9)
    expect(getRedlineBarStart(1, 10)).toBe(10)
    expect(getRedlineBarStart(0, 10)).toBe(0)
  })

  test('linear zones meet at the fill boundary in both orientations', () => {
    const rect = { x: 0, y: 0, width: 200, height: 40 }
    const horizontal = getLinearRedlineZoneRects(rect, 0.8, 'horizontal')
    expect(horizontal.normal.x + horizontal.normal.width).toBe(160)
    expect(horizontal.redline.x).toBe(160)

    const vertical = getLinearRedlineZoneRects({ x: 0, y: 0, width: 40, height: 200 }, 0.8, 'vertical')
    expect(vertical.normal.y).toBeCloseTo(40)
    expect(vertical.redline.y + vertical.redline.height).toBeCloseTo(40)
  })

  test('arc zones are closed wedges from the center', () => {
    const zones = getArcRedlineZonePaths({
      centerX: 80,
      centerY: 80,
      radius: 66,
      startAngle: 180,
      sweepAngle: 180,
      trackThickness: 14,
      startCornerRadius: 0,
      endCornerRadius: 0,
      fraction: 0.8,
    })
    expect(zones.normal.startsWith('M 80 80 L')).toBe(true)
    expect(zones.redline.startsWith('M 80 80 L')).toBe(true)
    expect(zones.normal.endsWith('Z')).toBe(true)
  })
})

describe('redline preview rendering', () => {
  test('linear continuous fill is split at the redline once the fill passes it', () => {
    render(<OverlayLinearGaugeWidget widget={makeLinear(SCALE)} activity={rpmActivity(11000)} previewSecond={0} globalScale={1} />)
    const svg = screen.getByTestId('linear-gauge-preview')
    expect(screen.getByTestId('linear-gauge-redline-empty-track')).toBeTruthy()
    const fills = [...svg.querySelectorAll('rect')].map((rect) => rect.getAttribute('fill'))
    expect(fills).toContain('#40e0d0')
    expect(fills).toContain('#ff0000')
  })

  test('linear fill below the redline never uses the redline color', () => {
    render(<OverlayLinearGaugeWidget widget={makeLinear(SCALE)} activity={rpmActivity(4000)} previewSecond={0} globalScale={1} />)
    const filledRects = [...screen.getByTestId('linear-gauge-preview').querySelectorAll('rect')].filter(
      (rect) => rect.getAttribute('fill-opacity') === '1' && rect.getAttribute('fill') === '#ff0000',
    )
    expect(filledRects).toHaveLength(0)
  })

  test('linear bars switch to the redline color from the redline bar on', () => {
    const widget = makeLinear({ ...SCALE, track_fill_style: 'bars', bar_count: 10, bar_gap: 0 })
    render(<OverlayLinearGaugeWidget widget={widget} activity={rpmActivity(11000)} previewSecond={0} globalScale={1} />)
    expect(screen.getAllByTestId('linear-gauge-bar-filled')).toHaveLength(8)
    expect(screen.getAllByTestId('linear-gauge-bar-redline-filled')).toHaveLength(2)
    expect(screen.getAllByTestId('linear-gauge-bar-redline-empty')).toHaveLength(2)
  })

  test('arc shows the redline on the empty track and on the fill', () => {
    render(<OverlayArcGaugeWidget widget={makeArc(SCALE)} activity={rpmActivity(11000)} previewSecond={0} globalScale={1} />)
    expect(screen.getByTestId('arc-gauge-redline-empty-track')).toHaveAttribute('fill', '#ff0000')
    expect(screen.getByTestId('arc-gauge-redline-filled-track')).toHaveAttribute('fill', '#ff0000')
    expect(screen.getByTestId('arc-gauge-filled-track')).toHaveAttribute('fill', '#40e0d0')
  })

  test('arc without a redline keeps its original test ids', () => {
    render(<OverlayArcGaugeWidget widget={makeArc()} activity={rpmActivity(5000)} previewSecond={0} globalScale={1} />)
    expect(screen.getByTestId('arc-gauge-empty-track')).toBeTruthy()
    expect(screen.queryByTestId('arc-gauge-redline-empty-track')).toBeNull()
  })
})

describe('gauge scale editor updates', () => {
  const defaults = { min: 0, max: 12000, redline: 10000 }

  test('only rpm and temperature metrics expose the controls', () => {
    expect(getGaugeScaleDefaults('rpm')).toMatchObject({ min: 0 })
    for (const type of ['temperature', 'core_temperature', 'coolant_temperature', 'intake_air_temperature']) {
      expect(getGaugeScaleDefaults(type)).not.toBeNull()
    }
    for (const type of ['speed', 'heartrate', 'power', 'throttle_position']) {
      expect(getGaugeScaleDefaults(type)).toBeNull()
    }
  })

  test('enabling and disabling the fixed scale writes and clears both bounds', () => {
    expect(buildGaugeScaleUpdate({}, defaults, { fixedScale: true })).toEqual({ scale_min: 0, scale_max: 12000 })
    expect(buildGaugeScaleUpdate({ scale_min: 0, scale_max: 12000 }, defaults, { fixedScale: false })).toEqual({
      scale_min: undefined,
      scale_max: undefined,
    })
  })

  test('rejects an inverted or non-numeric range', () => {
    const data = { scale_min: 0, scale_max: 12000 }
    expect(buildGaugeScaleUpdate(data, defaults, { scaleMax: '0' })).toEqual({})
    expect(buildGaugeScaleUpdate(data, defaults, { scaleMax: 'abc' })).toEqual({})
    expect(buildGaugeScaleUpdate(data, defaults, { scaleMax: '11000' })).toEqual({ scale_min: 0, scale_max: 11000 })
  })

  test('moves the redline inside the range when the scale shrinks past it', () => {
    const data = { scale_min: 0, scale_max: 12000, redline_value: 10000 }
    const patch = buildGaugeScaleUpdate(data, defaults, { scaleMax: '8000' })
    expect(patch.scale_max).toBe(8000)
    expect(patch.redline_value).toBeGreaterThan(0)
    expect(patch.redline_value).toBeLessThan(8000)
  })

  test('redline toggle fills a color and keeps the value inside a fixed scale', () => {
    const patch = buildGaugeScaleUpdate({ scale_min: 0, scale_max: 8000 }, defaults, { redline: true })
    expect(patch.redline_value).toBeGreaterThan(0)
    expect(patch.redline_value).toBeLessThan(8000)
    expect(patch.redline_color).toBe('#ff3b30')
    expect(buildGaugeScaleUpdate({ redline_value: 9000 }, defaults, { redline: false })).toEqual({ redline_value: undefined })
  })

  test('rejects a redline outside the fixed scale but accepts any value on the automatic scale', () => {
    expect(buildGaugeScaleUpdate({ scale_min: 0, scale_max: 8000 }, defaults, { redlineValue: '9000' })).toEqual({})
    expect(buildGaugeScaleUpdate({ scale_min: 0, scale_max: 8000 }, defaults, { redlineValue: '7000' })).toEqual({ redline_value: 7000 })
    expect(buildGaugeScaleUpdate({}, defaults, { redlineValue: '9000' })).toEqual({ redline_value: 9000 })
  })
})

describe('scale and redline survive template normalization and the backend config', () => {
  const sceneConfig = { width: 1920, height: 1080, fps: 30, updateRate: 1 }
  const rpmArc = (extra) => ({
    id: 'rpm-1',
    value: 'rpm',
    x: 10,
    y: 10,
    display_type: 'arc',
    display_variants: {
      arc: { width: 160, height: 160, arc_angle: 180, ...extra },
    },
  })

  test('arc, corner and linear variants keep the scale fields and drop unset ones', async () => {
    const { normalizeTemplateConfig } = await import('@/lib/template/template-normalization')
    const normalized = normalizeTemplateConfig(
      { scene: sceneConfig, backdrops: [], rasters: [], labels: [], plots: [], values: [rpmArc({ ...SCALE, bogus: 1 })] },
      {},
    )
    const variant = normalized.values[0].display_variants.arc
    expect(variant).toMatchObject(SCALE)
    expect(variant).not.toHaveProperty('bogus')

    const unset = normalizeTemplateConfig(
      { scene: sceneConfig, backdrops: [], rasters: [], labels: [], plots: [], values: [rpmArc({ scale_min: undefined, redline_value: undefined })] },
      {},
    )
    expect(unset.values[0].display_variants.arc).not.toHaveProperty('scale_min')
    expect(unset.values[0].display_variants.arc).not.toHaveProperty('redline_value')
  })

  test('the effective widget data flattens the active variant, scale included', async () => {
    const { getEffectiveWidgetData } = await import('@/lib/template/template-state')
    const data = getEffectiveWidgetData({ category: 'values', data: rpmArc(SCALE) }, { font_text: 'Arial.ttf', font_values: 'Arial.ttf', opacity: 1 })
    expect(data).toMatchObject(SCALE)
  })
})
