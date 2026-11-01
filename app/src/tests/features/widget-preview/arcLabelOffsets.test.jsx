import { render } from '@testing-library/react'
import { describe, expect, test } from 'vitest'
import { OverlayArcGaugeWidget } from '@/features/widget-preview/widgets/arc-gauge/ArcGaugePreview'
import { normalizeTemplateConfig } from '@/lib/template/template-normalization'
import { buildUniformResizeUpdate } from '@/features/overlay-editor/utils/widgetResizeScaling'

function makeArc(overrides = {}) {
  return {
    id: 'speed-arc',
    type: 'speed',
    category: 'values',
    data: {
      value: 'speed',
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
      show_min_max_labels: true,
      min_max_label_font: 'Arial.ttf',
      min_max_label_font_size: 12,
      min_max_label_color: '#ffffff',
      font: 'Arial.ttf',
      font_size: 30,
      color: '#ffffff',
      unit_color: '#ffffff',
      show_units: false,
      show_value: false,
      display_unit: 'kmh',
      decimals: 0,
      prefix: '',
      suffix: '',
      ...overrides,
    },
  }
}

const speedActivity = { sample_elapsed_seconds: [0, 1], trim_end_seconds: 1, speed: [10, 10] }

function labelPositions(overrides) {
  const { container } = render(<OverlayArcGaugeWidget widget={makeArc(overrides)} activity={speedActivity} previewSecond={0} globalScale={1} />)
  return [...container.querySelectorAll('text')].map((node) => ({
    text: node.textContent,
    x: Number(node.getAttribute('x') ?? node.querySelector('tspan')?.getAttribute('x')),
    y: Number(node.getAttribute('y') ?? node.querySelector('tspan')?.getAttribute('y')),
  }))
}

describe('min/max label offsets', () => {
  test('each label moves on its own', () => {
    const base = labelPositions({})
    const moved = labelPositions({ max_label_offset_x: -10, max_label_offset_y: 25 })
    expect(base).toHaveLength(2)
    expect(moved[0]).toEqual(base[0])
    expect(moved[1].x).toBeCloseTo(base[1].x - 10)
    expect(moved[1].y).toBeCloseTo(base[1].y + 25)

    const movedMin = labelPositions({ min_label_offset_y: 25 })
    expect(movedMin[0].y).toBeCloseTo(base[0].y + 25)
    expect(movedMin[1]).toEqual(base[1])
  })
})

describe('arc unit position and label offsets in templates', () => {
  const scene = { width: 1920, height: 1080, fps: 30, updateRate: 1 }
  const arcValue = (extra) => ({
    id: 'speed-1',
    value: 'speed',
    x: 10,
    y: 10,
    display_type: 'arc',
    display_variants: { arc: { width: 160, height: 160, arc_angle: 180, ...extra } },
  })

  test('arc variants keep the optional keys and drop unset ones', () => {
    const extra = { unit_position: 'right', min_label_offset_x: 4, min_label_offset_y: -6, max_label_offset_x: 8, max_label_offset_y: 12 }
    const normalized = normalizeTemplateConfig({ scene, backdrops: [], rasters: [], labels: [], plots: [], values: [arcValue(extra)] }, {})
    expect(normalized.values[0].display_variants.arc).toMatchObject(extra)

    const unset = normalizeTemplateConfig(
      { scene, backdrops: [], rasters: [], labels: [], plots: [], values: [arcValue({ unit_position: undefined, min_label_offset_x: undefined })] },
      {},
    )
    expect(unset.values[0].display_variants.arc).not.toHaveProperty('unit_position')
    expect(unset.values[0].display_variants.arc).not.toHaveProperty('min_label_offset_x')
  })

  test('resizing scales the label offsets that are set', () => {
    const widget = makeArc({ min_label_offset_x: 10, max_label_offset_y: -20 })
    widget.data.display_variants = { arc: { ...widget.data } }
    const update = buildUniformResizeUpdate(widget, 320)
    const variant = update.display_variants.arc
    expect(variant.min_label_offset_x).toBe(20)
    expect(variant.max_label_offset_y).toBe(-40)
    expect(variant).not.toHaveProperty('min_label_offset_y')
  })
})
