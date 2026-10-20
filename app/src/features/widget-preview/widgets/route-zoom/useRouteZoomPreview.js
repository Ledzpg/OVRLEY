import { useEffect, useMemo, useRef, useState } from 'react'
import { buildRouteZoomFrame, hasTauriRuntime } from '@/api/backend'
import { getDistanceProgressAtElapsed, getPreviewActivity, getWindowProgressAtTime } from '@/features/overlay-editor'
import { resolveExportRangeWindow } from '@/features/overlay-editor/utils/exportRange'
import useStore from '@/store/useStore'
import { buildPlotGeometryConfig } from '../../shared/usePlotPreviewGeometry'

/** Frames are requested this long after the last change, so dragging a slider does not queue downloads. */
const REQUEST_DELAY_MS = 150
/** Progress is rounded to this step: a frame differing by less than a pixel is the same frame. */
const PROGRESS_STEP = 1e-4

const NO_FRAME = Object.freeze({ image: null, failed: false })

/**
 * Loads the rendered close-up map frame for the preview second.
 *
 * Rust draws the whole widget (tiles, route, arrow and credit); this hook only chooses the frame the way
 * the route preview places its marker and keeps the latest answer.
 *
 * @param {object} params
 * @param {object} params.widget - Close-up map widget.
 * @param {object|null} params.activity - Stable parsed activity.
 * @param {number} params.previewSecond - Current preview time in seconds.
 * @param {number} params.globalScale - Scene scale.
 * @param {object|null} params.exportRange - Active export-range selection.
 * @returns {{ image: string|null, failed: boolean, placeholder: boolean }} Latest frame state.
 */
export function useRouteZoomPreview({ widget, activity, previewSecond, globalScale, exportRange }) {
  const [frame, setFrame] = useState(NO_FRAME)
  const config = useStore((state) => state.config)
  const globalDefaults = useStore((state) => state.globalDefaults)
  const widgetUpdateRate = useStore((state) => state.renderSettings.widgetUpdateRate)
  const data = widget.data
  const exportWindow = useMemo(
    () => resolveExportRangeWindow(activity, exportRange, data.show_full_activity),
    [activity, data.show_full_activity, exportRange],
  )
  const placeholder = getPreviewActivity(activity, previewSecond) === null

  const rawProgress = placeholder
    ? 0
    : exportWindow.active
      ? (getWindowProgressAtTime(activity, exportWindow, previewSecond) ?? 0)
      : getDistanceProgressAtElapsed(activity, previewSecond)
  const progress = Math.round(Math.min(1, Math.max(0, rawProgress)) / PROGRESS_STEP) * PROGRESS_STEP
  const dataSignature = JSON.stringify(data)
  const latest = useRef(null)

  useEffect(() => {
    latest.current = { activity, config, plotData: data, exportWindow, globalDefaults, globalScale, widgetUpdateRate }
  }, [activity, config, data, exportWindow, globalDefaults, globalScale, widgetUpdateRate])

  useEffect(() => {
    if (placeholder || !hasTauriRuntime()) {
      setFrame(NO_FRAME)
      return undefined
    }
    let cancelled = false
    const timer = setTimeout(() => {
      const geometryConfig = buildPlotGeometryConfig(latest.current)
      if (!geometryConfig) return
      buildRouteZoomFrame(geometryConfig, activity, progress)
        .then((image) => {
          if (!cancelled) setFrame({ image, failed: false })
        })
        .catch((error) => {
          console.error('Close-up map preview failed', error)
          if (!cancelled) setFrame({ image: null, failed: true })
        })
    }, REQUEST_DELAY_MS)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [activity, dataSignature, exportWindow, globalScale, placeholder, progress, widgetUpdateRate])

  return { ...frame, placeholder }
}
