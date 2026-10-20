/** Arc-shaped gauge SVG preview. */

import { getArcFilledTrackPath, getArcRoundedSegmentPath } from './trackPath'
import { useArcGaugePreviewPresentation } from './useArcGaugePreview'
import { PreviewSvgShadowBlurFilter, PreviewSvgText } from '../../shared/PreviewSvgComponents'
import { getStopColorAt } from '../../shared/gaugeZones'

/**
 * Renders a filled arc track path.
 * @param {object} props - SVG paint props plus arc geometry fields.
 * @returns {JSX.Element} Arc path or masked arc path.
 */
function ArcTrackPath({ fill, fillOpacity, dataTestId, ...geometry }) {
  return <path data-testid={dataTestId} d={getArcRoundedSegmentPath(geometry)} fill={fill} fillOpacity={fillOpacity} fillRule="evenodd" />
}

function ArcSegmentPaths({ geometries, dataTestId, ...paint }) {
  const paths = []
  for (let index = 0; index < geometries.length; index += 1) {
    paths.push(
      <path
        key={index}
        data-testid={dataTestId}
        d={getArcRoundedSegmentPath({ ...geometries[index], ...paint })}
        fill={paint.fill}
        fillOpacity={paint.fillOpacity}
        fillRule="evenodd"
        stroke={paint.stroke}
        strokeOpacity={paint.strokeOpacity}
        strokeWidth={paint.strokeWidth}
      />,
    )
  }
  return paths
}

function getArcSegmentInnerGeometry({ centerX, centerY, radius, startAngle, sweepAngle, trackThickness, borderThickness, cornerRadius }) {
  const direction = Math.sign(sweepAngle)
  const maxAngleOffsetDeg = Math.abs(sweepAngle) / 2
  const angleOffsetDeg = borderThickness > 0 && radius > 0 ? Math.min(((borderThickness / radius) * 180) / Math.PI, maxAngleOffsetDeg) : 0
  return {
    centerX,
    centerY,
    radius,
    startAngle: startAngle + direction * angleOffsetDeg,
    sweepAngle: sweepAngle - direction * angleOffsetDeg * 2,
    trackThickness: Math.max(0, trackThickness - borderThickness * 2),
    cornerRadius: Math.max(0, cornerRadius - borderThickness),
  }
}

function ArcSegmentMaskPaths({ geometries, borderThickness, cornerRadius }) {
  const paths = []
  for (let index = 0; index < geometries.length; index += 1) {
    const outerPath = getArcRoundedSegmentPath({ ...geometries[index], cornerRadius })
    const innerGeo = getArcSegmentInnerGeometry({ ...geometries[index], borderThickness, cornerRadius })
    const innerPath = getArcRoundedSegmentPath(innerGeo)
    paths.push(
      <g key={index}>
        <path d={outerPath} fill="#ffffff" fillOpacity={1} fillRule="evenodd" />
        <path d={innerPath} fill="#000000" fillOpacity={1} fillRule="evenodd" />
      </g>,
    )
  }
  return paths
}

function buildArcSegmentGeometry(trackGeometry, segments) {
  const geometries = []
  for (const segment of segments) geometries.push({ ...trackGeometry, ...segment })
  return geometries
}

/** Bars of a segmented arc fill, each in its band color, the redline color past the redline, or red on overrun. */
function ArcBarFill({ geometries, data, redline, overrun, fillStops, filledBarCount, filledOpacity, testIdPrefix }) {
  const bars = []
  for (let index = 0; index < filledBarCount; index += 1) {
    const inRedline = overrun != null || (redline != null && index >= redline.barStart)
    let color = data.track_filled_color
    if (overrun) color = overrun.color
    else if (inRedline) color = redline.color
    else if (fillStops) color = getStopColorAt(fillStops, (index + 0.5) / geometries.length)
    bars.push(
      <ArcSegmentPaths
        key={index}
        geometries={[geometries[index]]}
        cornerRadius={data.track_corner_radius}
        fill={color}
        fillOpacity={filledOpacity}
        dataTestId={`${testIdPrefix}-${inRedline ? 'bar-redline-filled' : 'bar-filled'}`}
      />,
    )
  }
  return bars
}

/** Continuous arc fill: band gradient, redline split, or the full redline fill on overrun. */
function ArcContinuousFill({ data, trackGeometry, redline, overrun, zoneWedges, ids, filledOpacity, testIdPrefix }) {
  const solid = (color, testId) => (
    <ArcTrackPath {...trackGeometry} cornerRadius={data.track_corner_radius} fill={color} fillOpacity={filledOpacity} dataTestId={testId} />
  )
  const normal = zoneWedges ? (
    <g clipPath={`url(#${ids.trackShapeId})`} data-testid={`${testIdPrefix}-zone-fill`}>
      {zoneWedges.map((wedge, index) => (
        <path key={index} d={wedge.d} fill={wedge.color} fillOpacity={filledOpacity} />
      ))}
    </g>
  ) : (
    solid(data.track_filled_color, `${testIdPrefix}-filled-track`)
  )
  return (
    <g clipPath={`url(#${ids.fillClipId})`}>
      {overrun ? (
        solid(overrun.color, `${testIdPrefix}-overrun-fill`)
      ) : redline?.fillPastRedline ? (
        <>
          <g clipPath={`url(#${ids.normalZoneId})`}>{normal}</g>
          <g clipPath={`url(#${ids.redlineZoneId})`}>{solid(redline.color, `${testIdPrefix}-redline-filled-track`)}</g>
        </>
      ) : (
        normal
      )}
    </g>
  )
}

function ArcGaugeTrack({
  data,
  trackGeometry,
  barLayout,
  filledBarCount,
  fillReveal,
  redline,
  fillStops,
  zoneWedges,
  overrun,
  opacity,
  shadow,
  maskPadding,
  widgetId,
}) {
  const segmented = barLayout != null
  const geometries = segmented ? buildArcSegmentGeometry(trackGeometry, barLayout.segments) : [trackGeometry]
  const maskId = `${data.display_type}-gauge-${widgetId}-border-mask`
  const fillClipId = `${data.display_type}-gauge-${widgetId}-fill-clip`
  const normalZoneId = `${data.display_type}-gauge-${widgetId}-zone-normal`
  const redlineZoneId = `${data.display_type}-gauge-${widgetId}-zone-redline`
  const trackShapeId = `${data.display_type}-gauge-${widgetId}-track-shape`
  const testIdPrefix = `${data.display_type}-gauge`
  const emptyOpacity = data.track_empty_opacity * opacity
  const filledOpacity = data.track_filled_opacity * opacity

  return (
    <>
      {data.track_border_thickness > 0 || fillReveal || redline ? (
        <defs>
          {zoneWedges ? (
            <clipPath id={trackShapeId}>
              <path
                d={getArcRoundedSegmentPath({ ...trackGeometry, cornerRadius: data.track_corner_radius })}
                fillRule="evenodd"
                clipRule="evenodd"
              />
            </clipPath>
          ) : null}
          {redline && !segmented ? (
            <>
              <clipPath id={normalZoneId}>
                <path d={redline.zones.normal} />
              </clipPath>
              <clipPath id={redlineZoneId}>
                <path d={redline.zones.redline} />
              </clipPath>
            </>
          ) : null}
          {data.track_border_thickness > 0 ? (
            <mask
              id={maskId}
              maskUnits="userSpaceOnUse"
              x={-maskPadding}
              y={-maskPadding}
              width={data.width + maskPadding * 2}
              height={data.height + maskPadding * 2}
              style={{ maskType: 'luminance' }}
            >
              <ArcSegmentMaskPaths geometries={geometries} borderThickness={data.track_border_thickness} cornerRadius={data.track_corner_radius} />
            </mask>
          ) : null}
          {fillReveal ? (
            <clipPath id={fillClipId}>
              <path
                data-testid={`${data.display_type}-gauge-fill-clip`}
                d={getArcFilledTrackPath({ ...trackGeometry, ...fillReveal })}
                fillRule="evenodd"
                clipRule="evenodd"
              />
            </clipPath>
          ) : null}
        </defs>
      ) : null}
      {shadow ? (
        <g transform={`translate(${shadow.distance} ${shadow.distance})`} filter={`url(#${data.display_type}-gauge-${widgetId}-shadow)`}>
          <g mask={data.track_border_thickness > 0 ? `url(#${maskId})` : undefined}>
            <ArcSegmentPaths geometries={geometries} cornerRadius={data.track_corner_radius} fill={shadow.color} fillOpacity={opacity} />
          </g>
        </g>
      ) : null}
      {redline && segmented ? (
        <>
          <ArcSegmentPaths
            geometries={geometries.slice(0, redline.barStart)}
            cornerRadius={data.track_corner_radius}
            fill={data.track_empty_color}
            fillOpacity={emptyOpacity}
            dataTestId={`${testIdPrefix}-bar-empty`}
          />
          <ArcSegmentPaths
            geometries={geometries.slice(redline.barStart)}
            cornerRadius={data.track_corner_radius}
            fill={redline.color}
            fillOpacity={emptyOpacity}
            dataTestId={`${testIdPrefix}-bar-redline-empty`}
          />
        </>
      ) : redline ? (
        <>
          <g clipPath={`url(#${normalZoneId})`}>
            <ArcSegmentPaths
              geometries={geometries}
              cornerRadius={data.track_corner_radius}
              fill={data.track_empty_color}
              fillOpacity={emptyOpacity}
              dataTestId={`${testIdPrefix}-empty-track`}
            />
          </g>
          <g clipPath={`url(#${redlineZoneId})`}>
            <ArcSegmentPaths
              geometries={geometries}
              cornerRadius={data.track_corner_radius}
              fill={redline.color}
              fillOpacity={emptyOpacity}
              dataTestId={`${testIdPrefix}-redline-empty-track`}
            />
          </g>
        </>
      ) : (
        <ArcSegmentPaths
          geometries={geometries}
          cornerRadius={data.track_corner_radius}
          fill={data.track_empty_color}
          fillOpacity={emptyOpacity}
          dataTestId={`${testIdPrefix}-${segmented ? 'bar-empty' : 'empty-track'}`}
        />
      )}
      {segmented ? (
        <ArcBarFill
          geometries={geometries}
          data={data}
          redline={redline}
          overrun={overrun}
          fillStops={fillStops}
          filledBarCount={filledBarCount}
          filledOpacity={filledOpacity}
          testIdPrefix={testIdPrefix}
        />
      ) : fillReveal ? (
        <ArcContinuousFill
          data={data}
          trackGeometry={trackGeometry}
          redline={redline}
          overrun={overrun}
          zoneWedges={zoneWedges}
          ids={{ fillClipId, normalZoneId, redlineZoneId, trackShapeId }}
          filledOpacity={filledOpacity}
          testIdPrefix={testIdPrefix}
        />
      ) : null}
      {data.track_border_thickness > 0 ? (
        <g mask={`url(#${maskId})`}>
          <ArcSegmentPaths
            geometries={geometries}
            cornerRadius={data.track_corner_radius}
            fill={data.track_border_color}
            fillOpacity={opacity}
            dataTestId={`${data.display_type}-gauge-${segmented ? 'bar-border' : 'border'}`}
          />
        </g>
      ) : null}
    </>
  )
}

/**
 * Renders the normalized arc or corner gauge preview.
 * @param {object} props - Normalized widget and preview state.
 * @returns {JSX.Element} Gauge SVG.
 */
export function OverlayArcGaugeWidget({ widget, activity, previewSecond, globalOpacity, globalScale, sceneStyle }) {
  const presentation = useArcGaugePreviewPresentation({ widget, activity, previewSecond, globalOpacity, sceneStyle })
  const {
    trackGeometry,
    innerModel,
    innerLayout,
    opacity,
    fillReveal,
    barLayout,
    filledBarCount,
    redline,
    fillStops,
    zoneWedges,
    overrun,
    ticks,
    minLabel,
    maxLabel,
    labels,
    labelFontFamily,
    shadow,
    trackShadow,
    maskPadding,
  } = presentation

  return (
    <svg
      width={widget.data.width * globalScale}
      height={widget.data.height * globalScale}
      viewBox={`0 0 ${widget.data.width} ${widget.data.height}`}
      className="block overflow-visible"
      data-testid={`${widget.data.display_type}-gauge-preview`}
    >
      {trackShadow ? <PreviewSvgShadowBlurFilter id={`${widget.data.display_type}-gauge-${widget.id}-shadow`} shadow={trackShadow} /> : null}
      <ArcGaugeTrack
        data={widget.data}
        trackGeometry={trackGeometry}
        barLayout={barLayout}
        filledBarCount={filledBarCount}
        fillReveal={fillReveal}
        redline={redline}
        fillStops={fillStops}
        zoneWedges={zoneWedges}
        overrun={overrun}
        opacity={opacity}
        shadow={trackShadow}
        maskPadding={maskPadding}
        widgetId={widget.id}
      />
      {ticks ? (
        <g data-testid={`${widget.data.display_type}-gauge-ticks`}>
          {ticks.lines.map((line, index) => (
            <line
              key={index}
              x1={line.x1}
              y1={line.y1}
              x2={line.x2}
              y2={line.y2}
              stroke={line.color}
              strokeWidth={widget.data.tick_thickness}
              strokeOpacity={opacity}
              data-testid={line.major ? 'arc-gauge-tick-major' : 'arc-gauge-tick-minor'}
            />
          ))}
          {ticks.labels.map((label, index) => (
            <PreviewSvgText
              key={index}
              text={label.text}
              x={label.x}
              baseline={label.baseline}
              color={label.color}
              fontFamily={labelFontFamily}
              fontSize={widget.data.tick_label_font_size}
              opacity={opacity}
            />
          ))}
        </g>
      ) : null}
      {widget.data.show_min_max_labels ? (
        <>
          <PreviewSvgText
            text={minLabel}
            x={labels.min.x}
            baseline={labels.min.baseline}
            color={widget.data.min_max_label_color}
            fontFamily={labelFontFamily}
            fontSize={widget.data.min_max_label_font_size}
            opacity={opacity}
            shadow={shadow}
            shadowFilterId={`${widget.data.display_type}-gauge-${widget.id}-label-shadow-min`}
            borderColor={sceneStyle?.border_color}
            borderThickness={sceneStyle?.border_thickness}
          />
          <PreviewSvgText
            text={maxLabel}
            x={labels.max.x}
            baseline={labels.max.baseline}
            color={widget.data.min_max_label_color}
            fontFamily={labelFontFamily}
            fontSize={widget.data.min_max_label_font_size}
            opacity={opacity}
            shadow={shadow}
            shadowFilterId={`${widget.data.display_type}-gauge-${widget.id}-label-shadow-max`}
            borderColor={sceneStyle?.border_color}
            borderThickness={sceneStyle?.border_thickness}
          />
        </>
      ) : null}
      {innerLayout.unit ? (
        <PreviewSvgText
          text={innerModel.unitText}
          x={innerLayout.unit.x}
          baseline={innerLayout.unit.baseline}
          color={widget.data.unit_color}
          fontFamily={innerModel.fontFamily}
          fontSize={innerLayout.unit.fontSize}
          opacity={opacity}
          shadow={shadow}
          shadowFilterId={`${widget.data.display_type}-gauge-${widget.id}-unit-shadow`}
          borderColor={sceneStyle?.border_color}
          borderThickness={sceneStyle?.border_thickness}
        />
      ) : null}
      <PreviewSvgText
        text={innerModel.valueText}
        x={innerLayout.value.x}
        baseline={innerLayout.value.baseline}
        color={widget.data.color}
        fontFamily={innerModel.fontFamily}
        fontSize={widget.data.font_size}
        opacity={opacity}
        shadow={shadow}
        shadowFilterId={`${widget.data.display_type}-gauge-${widget.id}-value-shadow`}
        borderColor={sceneStyle?.border_color}
        borderThickness={sceneStyle?.border_thickness}
      />
    </svg>
  )
}
