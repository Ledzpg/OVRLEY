/**
 * Close-up route map: defaults, editor update builders, persistence, the drawer entry and the preview.
 */

import { beforeEach, describe, expect, test, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { OverlayRouteZoomWidget } from '@/features/widget-preview/widgets/route-zoom/RouteZoomPreview'
import { buildZoomUpdate, ZOOM_MAX, ZOOM_MIN } from '@/features/widget-editor/components/routeZoomUpdates'
import { buildMapUpdate } from '@/features/widget-editor/components/routeMapUpdates'
import { createPlotDefaults } from '@/features/widget-editor/utils/widgetUtils'
import { WidgetDrawerContent as WidgetDrawerView } from '@/features/widget-drawer'
import useWidgetDraftState from '@/features/overlay-editor/hooks/useWidgetDraftState'
import { COURSE_ZOOM_PLOT_DEFAULTS, WIDGET_TYPE_DEFINITIONS } from '@/lib/widget/standard-widgets'
import { COURSE_ZOOM_PLOT_KEYS } from '@/lib/template/template-constants'
import { normalizeTemplateConfig } from '@/lib/template/template-normalization'
import { cloneSerializable, DEFAULT_CONFIG } from '@/store/store-utils'
import useStore from '@/store/useStore'
import * as backend from '@/api/backend'

vi.mock('@/api/backend', async (importOriginal) => ({
  ...(await importOriginal()),
  hasTauriRuntime: vi.fn(() => false),
  buildRouteZoomFrame: vi.fn(),
}))

const widget = () => ({ id: 'zoom-1', type: 'course_zoom', category: 'plots', data: { ...COURSE_ZOOM_PLOT_DEFAULTS } })

function WidgetDrawerContent() {
  const widgetLiveEdits = useWidgetDraftState()
  return <WidgetDrawerView widgetLiveEdits={widgetLiveEdits} />
}

beforeEach(() => {
  backend.hasTauriRuntime.mockReturnValue(false)
  backend.buildRouteZoomFrame.mockReset()
  useStore.setState({ config: cloneSerializable(DEFAULT_CONFIG), leftDrawerPinned: false, leftDrawerVisible: true, selectedWidgetId: null })
})

describe('close-up map definition', () => {
  test('is a plot widget in the general category with its own name', () => {
    expect(WIDGET_TYPE_DEFINITIONS.course_zoom).toMatchObject({ category: 'general', nameKey: 'widgets.types.course_zoom.name' })
    expect(COURSE_ZOOM_PLOT_DEFAULTS.value).toBe('course_zoom')
  })

  test('defaults carry every field the renderer requires, with a map from a keyless provider', () => {
    const required = [
      'zoom',
      'corner_radius',
      'rotate_with_heading',
      'show_full_activity',
      'map_tile_url',
      'map_attribution',
      'map_opacity',
      'map_zoom_offset',
      'completed_line_width',
      'completed_line_color',
      'completed_line_opacity',
      'remaining_line_width',
      'remaining_line_color',
      'remaining_line_opacity',
      'marker_size',
      'marker_color',
      'marker_opacity',
      'marker_outline_color',
      'marker_outline_width',
    ]
    for (const field of required) expect(COURSE_ZOOM_PLOT_DEFAULTS).toHaveProperty(field)
    expect(COURSE_ZOOM_PLOT_DEFAULTS.map_tile_url).toContain('tile.openstreetmap.org')
    expect(COURSE_ZOOM_PLOT_DEFAULTS.zoom).toBeGreaterThanOrEqual(ZOOM_MIN)
    expect(COURSE_ZOOM_PLOT_DEFAULTS.zoom).toBeLessThanOrEqual(ZOOM_MAX)
  })

  test('creating the widget keeps the defaults and follows the global opacity', () => {
    expect(createPlotDefaults('course_zoom', { opacity: 0.8 })).toEqual({ ...COURSE_ZOOM_PLOT_DEFAULTS, opacity: 0.8 })
  })

  test('a saved template keeps every field and none of the route map fields', () => {
    const plot = { ...COURSE_ZOOM_PLOT_DEFAULTS, id: 'zoom-1', zoom: 17, rotate_with_heading: true, rotation: 33, shadow_strength: 4 }
    const [kept] = normalizeTemplateConfig({ scene: {}, rasters: [], plots: [plot] }, null).plots
    for (const key of COURSE_ZOOM_PLOT_KEYS) expect(kept[key]).toEqual(plot[key])
    expect(kept).not.toHaveProperty('rotation')
    expect(kept).not.toHaveProperty('shadow_strength')
    expect(kept).not.toHaveProperty('simplify_tolerance_px')
  })
})

describe('close-up map controls', () => {
  test('zoom stays inside what the renderer accepts', () => {
    expect(buildZoomUpdate({ field: 'zoom', value: '16.5' })).toEqual({ zoom: 16.5 })
    expect(buildZoomUpdate({ field: 'zoom', value: ZOOM_MIN - 1 })).toEqual({})
    expect(buildZoomUpdate({ field: 'zoom', value: ZOOM_MAX + 1 })).toEqual({})
    expect(buildZoomUpdate({ field: 'zoom', value: '' })).toEqual({})
  })

  test('sizes must be positive where the renderer needs it and opacities are percents', () => {
    expect(buildZoomUpdate({ field: 'marker_size', value: 0 })).toEqual({})
    expect(buildZoomUpdate({ field: 'completed_line_width', value: 0 })).toEqual({})
    expect(buildZoomUpdate({ field: 'marker_outline_width', value: 0 })).toEqual({ marker_outline_width: 0 })
    expect(buildZoomUpdate({ field: 'corner_radius', value: -1 })).toEqual({})
    expect(buildZoomUpdate({ field: 'marker_opacity', value: 60 })).toEqual({ marker_opacity: 60 })
    expect(buildZoomUpdate({ field: 'marker_opacity', value: 101 })).toEqual({})
    expect(buildZoomUpdate({ field: 'unknown', value: 3 })).toEqual({})
  })

  test('the map style, credit, opacity and detail reuse the route map builder', () => {
    const data = COURSE_ZOOM_PLOT_DEFAULTS
    expect(buildMapUpdate(data, { preset: 'opentopomap' }).map_tile_url).toContain('opentopomap')
    expect(buildMapUpdate(data, { zoomOffset: 1 })).toEqual({ map_zoom_offset: 1 })
    expect(buildMapUpdate(data, { opacity: 50 })).toEqual({ map_opacity: 50 })
  })
})

describe('close-up map in the drawer', () => {
  test('is offered as a widget and added with its defaults', async () => {
    const user = userEvent.setup()
    render(<WidgetDrawerContent />)

    await user.click(screen.getByText('Map+').closest('button'))

    const [plot] = useStore.getState().config.plots
    expect(plot).toMatchObject({ value: 'course_zoom', zoom: COURSE_ZOOM_PLOT_DEFAULTS.zoom, marker_color: COURSE_ZOOM_PLOT_DEFAULTS.marker_color })
    expect(plot.id).toMatch(/^widget-\d+$/)
    expect(useStore.getState().selectedWidgetId).toBe(plot.id)
  })
})

describe('close-up map preview', () => {
  const activity = {
    sample_elapsed_seconds: [0, 1],
    trim_end_seconds: 1,
    sample_distance_progress: [0, 1],
    sample_course_points: [
      [1, 1],
      [1.001, 1.001],
    ],
  }

  test('without an activity it shows a placeholder asking for one', () => {
    render(<OverlayRouteZoomWidget widget={widget()} activity={null} previewSecond={0} globalOpacity={1} globalScale={1} exportRange={null} />)
    expect(screen.getByText('Load an activity to see the map')).toBeInTheDocument()
    expect(backend.buildRouteZoomFrame).not.toHaveBeenCalled()
  })

  test('shows the frame rendered by the backend', async () => {
    backend.hasTauriRuntime.mockReturnValue(true)
    backend.buildRouteZoomFrame.mockResolvedValue('data:image/png;base64,AAAA')
    const { container } = render(
      <OverlayRouteZoomWidget widget={widget()} activity={activity} previewSecond={0.5} globalOpacity={1} globalScale={1} exportRange={null} />,
    )

    await waitFor(() => expect(container.querySelector('image')?.getAttribute('href')).toBe('data:image/png;base64,AAAA'))
    const [config, sentActivity, progress] = backend.buildRouteZoomFrame.mock.calls[0]
    expect(config.plots[0]).toMatchObject({ value: 'course_zoom', zoom: COURSE_ZOOM_PLOT_DEFAULTS.zoom })
    expect(sentActivity).toBe(activity)
    expect(progress).toBeGreaterThanOrEqual(0)
    expect(progress).toBeLessThanOrEqual(1)
  })

  test('reports a failed frame instead of hiding it', async () => {
    backend.hasTauriRuntime.mockReturnValue(true)
    backend.buildRouteZoomFrame.mockRejectedValue(new Error('offline'))
    vi.spyOn(console, 'error').mockImplementation(() => {})
    render(<OverlayRouteZoomWidget widget={widget()} activity={activity} previewSecond={0} globalOpacity={1} globalScale={1} exportRange={null} />)

    expect(await screen.findByText('Map unavailable')).toBeInTheDocument()
  })
})
