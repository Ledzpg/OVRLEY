/**
 * Renders the overlay route widget SVG preview â€” draws remaining/completed
 * polylines, shadow layers, and marker indicators for the course route.
 *
 * All data is received via props; no store access.
 *
 * @param {object} props
 * @param {object} props.widget - Widget configuration object.
 * @param {object|null} props.activity - Stable parsed activity used to prepare geometry and display activity data.
 * @param {number} props.previewSecond - Current preview time in seconds.
 * @param {number} props.globalOpacity - Global opacity multiplier.
 * @param {number} props.globalScale - Global scale multiplier.
 * @param {object} props.sceneStyle - Scene style object (shadow, border).
 * @param {object} props.exportRange - Export range configuration.
 * @returns {JSX.Element} SVG element for route preview.
 */

import { getWidgetOpacity } from '../../shared/textMeasurement'
import { getTextShadowParts } from '../../shared/shadow'
import { sanitizeSvgId } from '../../shared/svgPreviewUtils'
import { PreviewMarkerLayers, PreviewPolylineShadow, PreviewSvgShadowBlurFilter } from '../../shared/PreviewSvgComponents'
import { useTranslation } from 'react-i18next'
import { useRoutePreview } from './useRoutePreview'

const ATTRIBUTION_FONT_SIZE = 11
const ATTRIBUTION_PADDING = 3

export function OverlayRouteWidget({ widget, activity, previewSecond, globalOpacity, globalScale, sceneStyle, exportRange }) {
  const { t } = useTranslation()
  const previewModel = useRoutePreview({ widget, activity, previewSecond, globalScale, exportRange })

  if (!previewModel) {
    return null
  }

  const { style, geometry } = previewModel
  const shadow = getTextShadowParts(sceneStyle)
  const shadowFilterId = sanitizeSvgId(`${widget.id}-route-shadow-blur`)

  return (
    <svg
      width={widget.data.width}
      height={widget.data.height}
      viewBox={`0 0 ${widget.data.width} ${widget.data.height}`}
      className="block h-full w-full"
      style={{ opacity: getWidgetOpacity(widget.data, globalOpacity) }}
    >
      <PreviewSvgShadowBlurFilter id={shadowFilterId} shadow={shadow} />
      {geometry.mapImage ? (
        <image href={geometry.mapImage} x={0} y={0} width={widget.data.width} height={widget.data.height} preserveAspectRatio="none" />
      ) : null}
      <g>
        <PreviewPolylineShadow
          points={geometry.remainingSvgPoints}
          shadow={shadow}
          blurFilterId={shadowFilterId}
          strokeWidth={widget.data.remaining_line_width}
          strokeOpacity={style.remainingLineOpacity}
          rotation={widget.data.rotation}
        />
        <polyline
          fill="none"
          stroke={widget.data.remaining_line_color}
          strokeOpacity={style.remainingLineOpacity}
          strokeWidth={widget.data.remaining_line_width}
          strokeLinejoin="round"
          strokeLinecap="round"
          points={geometry.remainingSvgPoints}
        />
        <polyline
          fill="none"
          stroke={widget.data.completed_line_color}
          strokeOpacity={style.completedLineOpacity}
          strokeWidth={widget.data.completed_line_width}
          strokeLinejoin="round"
          strokeLinecap="round"
          points={geometry.completedSvgPoints}
        />
      </g>
      <PreviewMarkerLayers layers={style.markerLayers} point={geometry.markerPoint} />
      {geometry.mapImage ? <MapAttribution text={widget.data.map_attribution} width={widget.data.width} height={widget.data.height} /> : null}
      {geometry.mapFailed ? (
        <text x={8} y={widget.data.height - 8} fill="#ff6b6b" fontSize={12}>
          {t('widget-editor.routeMapUnavailable', 'Map unavailable')}
        </text>
      ) : null}
    </svg>
  )
}

/** Credit in the bottom-right corner, matching the one the renderer draws into the map. */
function MapAttribution({ text, width, height }) {
  const boxWidth = Math.min(width, text.length * ATTRIBUTION_FONT_SIZE * 0.55 + ATTRIBUTION_PADDING * 2)
  const boxHeight = ATTRIBUTION_FONT_SIZE + ATTRIBUTION_PADDING * 2
  return (
    <g>
      <rect x={width - boxWidth} y={height - boxHeight} width={boxWidth} height={boxHeight} fill="#ffffff" fillOpacity={0.75} />
      <text x={width - boxWidth + ATTRIBUTION_PADDING} y={height - ATTRIBUTION_PADDING - 2} fill="#333333" fontSize={ATTRIBUTION_FONT_SIZE}>
        {text}
      </text>
    </g>
  )
}
