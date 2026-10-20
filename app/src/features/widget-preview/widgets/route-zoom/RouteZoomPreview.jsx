/**
 * Renders the close-up route map preview: the frame drawn by the Rust renderer (tiles, route, arrow and credit),
 * so the preview shows exactly what the video will contain.
 *
 * @param {object} props
 * @param {object} props.widget - Widget configuration object.
 * @param {object|null} props.activity - Stable parsed activity.
 * @param {number} props.previewSecond - Current preview time in seconds.
 * @param {number} props.globalOpacity - Global opacity multiplier.
 * @param {number} props.globalScale - Global scale multiplier.
 * @param {object} props.exportRange - Export range configuration.
 * @returns {JSX.Element} SVG element for the close-up map preview.
 */

import { useTranslation } from 'react-i18next'
import { getWidgetOpacity } from '../../shared/textMeasurement'
import { useRouteZoomPreview } from './useRouteZoomPreview'

export function OverlayRouteZoomWidget({ widget, activity, previewSecond, globalOpacity, globalScale, exportRange }) {
  const { t } = useTranslation()
  const { image, failed, placeholder } = useRouteZoomPreview({ widget, activity, previewSecond, globalScale, exportRange })
  const { width, height, corner_radius: cornerRadius } = widget.data

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      className="block h-full w-full"
      style={{ opacity: getWidgetOpacity(widget.data, globalOpacity) }}
      data-testid="route-zoom-preview"
    >
      {image ? (
        <image href={image} x={0} y={0} width={width} height={height} preserveAspectRatio="none" />
      ) : (
        <rect
          x={1}
          y={1}
          width={width - 2}
          height={height - 2}
          rx={cornerRadius}
          fill="#888888"
          fillOpacity={0.25}
          stroke="#888888"
          strokeDasharray="6 4"
        />
      )}
      {!image && (failed || placeholder) ? (
        <text x={width / 2} y={height / 2} textAnchor="middle" fill={failed ? '#ff6b6b' : '#cccccc'} fontSize={14}>
          {failed
            ? t('widget-editor.routeMapUnavailable', 'Map unavailable')
            : t('widget-editor.routeZoomNeedsActivity', 'Load an activity to see the map')}
        </text>
      ) : null}
    </svg>
  )
}
