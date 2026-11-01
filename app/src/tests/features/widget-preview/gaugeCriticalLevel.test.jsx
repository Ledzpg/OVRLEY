import { render, screen } from '@testing-library/react'
import { describe, expect, test } from 'vitest'
import { OverlayArcGaugeWidget } from '@/features/widget-preview/widgets/arc-gauge/ArcGaugePreview'
import { OverlayLinearGaugeWidget } from '@/features/widget-preview/widgets/linear-gauge/LinearGaugePreview'
import { getArcGaugeLayout } from '@/features/widget-preview/widgets/arc-gauge/geometry'
import {
  fillEntersRedline,
  getArcRedlineZonePaths,
  getGaugeRedline,
  getLinearRedlineZoneRects,
  isFractionInRedline,
  isRedlineBar,
  splitRedlineBars,
} from '@/features/widget-preview/shared/gaugeRedline'
import { getBandRedline, getFillStops, isRedlineOverrun } from '@/features/widget-preview/shared/gaugeZones'
import { getStandardMetricRedlineZone } from '@/lib/widget/standard-metrics'

const FUEL = { scale_min: 0, scale_max: 20, redline_value: 5, redline_color: '#ff0000' }

function makeFuelArc(overrides = {}) {
  return {
    id: 'fuel-arc',
    type: 'fuel_level',
    category: 'values',
    data: {
      value: 'fuel_level',
      display_type: 'arc',
      opacity: 1,
      width: 160,
      height: 160,
      rotation: 0,
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
      show_value: false,
      display_unit: 'l',
      fuel_tank_capacity: 20,
      decimals: 0,
      ...overrides,
    },
  }
}

const fuelActivity = (value) => ({ sample_elapsed_seconds: [0, 1], trim_end_seconds: 1, fuel_level: [value, value] })

describe('critical level catalog', () => {
  test('fuel is a critical level (below) and engine speed a plain redline (above)', () => {
    expect(getStandardMetricRedlineZone('fuel_level')).toBe('below')
    expect(getStandardMetricRedlineZone('rpm')).toBe('above')
    expect(getStandardMetricRedlineZone('speed')).toBe('above')
  })
})

describe('critical level geometry', () => {
  const redline = { fraction: 0.25, color: '#ff0000', below: true, barStart: 3 }

  test('the redline records which side it marks', () => {
    expect(getGaugeRedline({ ...FUEL, value: 'fuel_level' }, { min: 0, max: 20 })).toMatchObject({ fraction: 0.25, below: true })
    expect(getGaugeRedline({ ...FUEL, value: 'rpm' }, { min: 0, max: 20 })).toMatchObject({ below: false })
  })

  test('positions and fills are tested against the start side', () => {
    expect(isFractionInRedline(redline, 0.1)).toBe(true)
    expect(isFractionInRedline(redline, 0.5)).toBe(false)
    expect(fillEntersRedline(redline, 0.01)).toBe(true)
    expect(fillEntersRedline(redline, 0)).toBe(false)
  })

  test('bars before the split are the critical ones', () => {
    expect([0, 1, 2, 3, 4].map((index) => isRedlineBar(redline, index))).toEqual([true, true, true, false, false])
    expect(splitRedlineBars(['a', 'b', 'c', 'd', 'e'], redline)).toEqual({ normal: ['d', 'e'], redline: ['a', 'b', 'c'] })
  })

  test('linear zones swap sides', () => {
    const rect = { x: 0, y: 0, width: 200, height: 40 }
    const zones = getLinearRedlineZoneRects(rect, 0.25, 'horizontal', true)
    expect(zones.redline.x + zones.redline.width).toBe(50)
    expect(zones.normal.x).toBe(50)
  })

  test('arc zones swap sides', () => {
    const geometry = {
      centerX: 80,
      centerY: 80,
      radius: 66,
      startAngle: 180,
      sweepAngle: 180,
      trackThickness: 14,
      startCornerRadius: 0,
      endCornerRadius: 0,
      fraction: 0.25,
    }
    const above = getArcRedlineZonePaths(geometry)
    const below = getArcRedlineZonePaths({ ...geometry, below: true })
    expect(below.redline).toBe(above.normal)
    expect(below.normal).toBe(above.redline)
  })

  test('a critical level fills completely once the value drops to it', () => {
    const data = { ...FUEL, value: 'fuel_level', redline_fill_all: true }
    expect(isRedlineOverrun(data, 5)).toBe(true)
    expect(isRedlineOverrun(data, 2)).toBe(true)
    expect(isRedlineOverrun(data, 6)).toBe(false)
    expect(isRedlineOverrun({ ...data, value: 'rpm' }, 4)).toBe(false)
    expect(isRedlineOverrun({ ...data, value: 'rpm' }, 5)).toBe(true)
  })

  test('fill bands never end in a critical level', () => {
    const zones = [{ start: 0, color: '#00ff00', end_follows_next: true }]
    const critical = getBandRedline({ ...FUEL, value: 'fuel_level' })
    expect(critical).toEqual({ value: undefined, color: undefined })
    const stops = getFillStops(zones, { min: 0, max: 20 }, critical.value, critical.color)
    expect(stops.at(-1)).toMatchObject({ fraction: 1, color: '#00ff00' })
  })

  test('arc layout carries the side', () => {
    const layout = getArcGaugeLayout({ ...makeFuelArc(FUEL).data }, 10, [10, 10])
    expect(layout.redline).toMatchObject({ fraction: 0.25, below: true })
  })
})

describe('critical level preview rendering', () => {
  test('arc marks the start of the empty track and paints the fill red inside it', () => {
    render(<OverlayArcGaugeWidget widget={makeFuelArc(FUEL)} activity={fuelActivity(12)} previewSecond={0} globalScale={1} />)
    expect(screen.getByTestId('arc-gauge-redline-empty-track')).toHaveAttribute('fill', '#ff0000')
    expect(screen.getByTestId('arc-gauge-redline-filled-track')).toHaveAttribute('fill', '#ff0000')
  })

  test('arc with show_value off draws only the bars, with it on the number', () => {
    const { container, rerender } = render(
      <OverlayArcGaugeWidget widget={makeFuelArc(FUEL)} activity={fuelActivity(12)} previewSecond={0} globalScale={1} />,
    )
    expect(container.querySelectorAll('text')).toHaveLength(0)
    rerender(
      <OverlayArcGaugeWidget widget={makeFuelArc({ ...FUEL, show_value: true })} activity={fuelActivity(12)} previewSecond={0} globalScale={1} />,
    )
    expect(container.querySelectorAll('text').length).toBeGreaterThan(0)
  })

  test('arc fill-all paints the whole track once the level is critical', () => {
    const widget = makeFuelArc({ ...FUEL, redline_fill_all: true })
    render(<OverlayArcGaugeWidget widget={widget} activity={fuelActivity(3)} previewSecond={0} globalScale={1} />)
    expect(screen.getByTestId('arc-gauge-overrun-fill')).toHaveAttribute('fill', '#ff0000')
    expect(screen.queryByTestId('arc-gauge-filled-track')).toBeNull()
  })

  test('linear bars: the first bars are the critical ones', () => {
    const widget = {
      id: 'fuel-linear',
      type: 'fuel_level',
      category: 'values',
      data: {
        ...makeFuelArc(FUEL).data,
        display_type: 'linear',
        width: 200,
        height: 40,
        orientation: 'horizontal',
        track_fill_style: 'bars',
        bar_count: 10,
        bar_gap: 0,
      },
    }
    render(<OverlayLinearGaugeWidget widget={widget} activity={fuelActivity(20)} previewSecond={0} globalScale={1} />)
    expect(screen.getAllByTestId('linear-gauge-bar-redline-filled')).toHaveLength(3)
    expect(screen.getAllByTestId('linear-gauge-bar-filled')).toHaveLength(7)
  })
})
