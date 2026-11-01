/** Text and unit layout for the inner presentation of an arc-shaped gauge. */

export const ARC_INNER_WIDGET_LINE_HEIGHT = 0.92
export const ARC_INNER_WIDGET_UNIT_RATIO = 0.28
export const ARC_INNER_WIDGET_MIN_UNIT_FONT_SIZE = 12
export const ARC_INNER_WIDGET_GAP_PX = 4

/**
 * Calculates the inner value/unit layout from normalized gauge data.
 * A unit above or below the value stacks with it as one centered group; a unit beside it sits against the value's
 * left or right edge on the same baseline, the value staying centered on the anchor.
 * @param {object} data - Normalized gauge data (`unit_position` unset means below).
 * @param {object} layout - Computed arc or corner layout.
 * @param {object} model - Formatted and measured inner content.
 * @returns {object} Positioned value and optional unit layout.
 */
export function getArcInnerWidgetLayout(data, layout, model) {
  const centerX = data.display_type === 'corner' ? layout.innerAnchor.x : layout.centerX
  const centerY = data.display_type === 'corner' ? layout.innerAnchor.y : layout.centerY
  const position = data.unit_position ?? 'below'
  const stacked = position === 'above' || position === 'below'
  const unitFontSize = Math.max(data.font_size * ARC_INNER_WIDGET_UNIT_RATIO, ARC_INNER_WIDGET_MIN_UNIT_FONT_SIZE)
  const valueLineHeight = data.font_size * ARC_INNER_WIDGET_LINE_HEIGHT
  const unitLineHeight = unitFontSize * ARC_INNER_WIDGET_LINE_HEIGHT
  const gap = Math.max(data.font_size * 0.08, ARC_INNER_WIDGET_GAP_PX)
  const unitVisible = model.unitText !== ''
  const stackedUnit = unitVisible && stacked
  const groupHeight = stackedUnit ? valueLineHeight + gap + unitLineHeight : valueLineHeight
  const groupTop = centerY + data.inner_widget_offset_y - groupHeight * 0.5
  const valueTop = stackedUnit && position === 'above' ? groupTop + unitLineHeight + gap : groupTop
  const contentCenterX = centerX + data.inner_widget_offset_x
  const valueX = contentCenterX - model.valueMeasure.width * 0.5
  const valueBaseline = valueTop + (valueLineHeight - model.valueVerticalMeasure.glyphHeight) * 0.5 + model.valueVerticalMeasure.ascent

  let unit = null
  if (stackedUnit) {
    const unitTop = position === 'above' ? groupTop : groupTop + valueLineHeight + gap
    unit = {
      x: contentCenterX - (model.unitMeasure.boundsLeft + model.unitMeasure.boundsRight) * 0.5,
      top: unitTop,
      baseline: unitTop + (unitLineHeight - model.unitMeasure.glyphHeight) * 0.5 + model.unitMeasure.ascent,
      fontSize: unitFontSize,
      lineHeight: unitLineHeight,
    }
  } else if (unitVisible) {
    const besideGap = Math.max(data.font_size * 0.08, ARC_INNER_WIDGET_GAP_PX * 2)
    const valueWidth = data.show_value ? model.valueMeasure.width : 0
    unit = {
      x:
        position === 'left' ? contentCenterX - valueWidth * 0.5 - besideGap - model.unitMeasure.width : contentCenterX + valueWidth * 0.5 + besideGap,
      // Shares the value's baseline.
      top: valueBaseline - ((unitLineHeight - model.unitMeasure.glyphHeight) * 0.5 + model.unitMeasure.ascent),
      baseline: valueBaseline,
      fontSize: unitFontSize,
      lineHeight: unitLineHeight,
    }
  }

  return {
    centerX: contentCenterX,
    groupTop,
    groupHeight,
    value: {
      x: valueX,
      top: valueTop,
      baseline: valueBaseline,
      lineHeight: valueLineHeight,
    },
    unit,
  }
}
