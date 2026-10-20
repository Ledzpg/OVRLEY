/**
 * Editor for the close-up route map widget: a map that follows the current position.
 */

import { Map as MapIcon, Navigation, Palette } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Label } from '@/components/ui/label'
import { SectionHeading } from '@/components/ui/section-heading'
import RouteMapBackgroundSection from './RouteMapBackgroundSection'
import { buildZoomUpdate, ZOOM_MAX, ZOOM_MIN, ZOOM_STEP } from './routeZoomUpdates'
import { DimensionsSection } from './widgetEditorSections'
import { ColorField, SliderField, ToggleField } from './widgetFormControls'

/**
 * Renders the close-up route map editor.
 *
 * @param {object} props - Component props.
 * @param {object} props.widget - Widget being edited.
 * @param {Function} props.updateWidgetData - Merges a patch into the widget data.
 * @param {Function} props.setNumericField - Sets a validated numeric field.
 * @returns {JSX.Element} Rendered component output.
 */
export default function RouteZoomWidgetEditor({ widget, updateWidgetData, setNumericField }) {
  const { t } = useTranslation()
  const data = widget.data
  const set = (field) => (value) => updateWidgetData(widget.id, buildZoomUpdate({ field, value }))
  const slider = (field, label, { min, max, step = 1, suffix = 'px', display = String } = {}) => (
    <SliderField
      editable
      label={label}
      value={data[field]}
      min={min}
      max={max}
      step={step}
      valueDisplay={display(data[field])}
      suffix={suffix}
      onSliderChange={set(field)}
    />
  )

  return (
    <>
      <DimensionsSection widget={widget} setNumericField={setNumericField} />

      <div className="space-y-4">
        <SectionHeading icon={MapIcon} title={t('widget-editor.routeZoomView', 'View')} />
        {slider('zoom', t('widget-editor.routeZoomLevel', 'Zoom'), {
          min: ZOOM_MIN,
          max: ZOOM_MAX,
          step: ZOOM_STEP,
          suffix: '',
          display: (value) => value.toFixed(1),
        })}
        <div className="grid grid-cols-2 gap-4">
          {slider('corner_radius', t('widget-editor.routeZoomCorner', 'Corner Radius'), { min: 0, max: 80 })}
          <div className="flex items-center justify-between gap-2 pl-1 pt-2 pb-2">
            <Label className="p-0 text-[9px] text-muted-foreground uppercase font-bold">
              {t('widget-editor.routeZoomRotate', 'Rotate With Heading')}
            </Label>
            <ToggleField
              checked={data.rotate_with_heading}
              onCheckedChange={(checked) => updateWidgetData(widget.id, { rotate_with_heading: checked })}
            />
          </div>
        </div>
        <div className="flex items-center justify-between gap-2 pl-1 pt-2 pb-2">
          <Label className="p-0 text-[9px] text-muted-foreground uppercase font-bold">
            {t('widget-editor.showFullActivity', 'Show Full Activity')}
          </Label>
          <ToggleField
            checked={data.show_full_activity}
            onCheckedChange={(checked) => updateWidgetData(widget.id, { show_full_activity: checked })}
          />
        </div>
      </div>

      <RouteMapBackgroundSection widget={widget} updateWidgetData={updateWidgetData} alwaysOn />

      <div className="space-y-4">
        <SectionHeading icon={Palette} title={t('widget-editor.lineStyling', 'Line Styling')} />
        <div className="grid grid-cols-2 gap-4">
          {slider('completed_line_width', t('widget-editor.finishedThickness', 'Finished Thickness'), { min: 1, max: 30 })}
          {slider('remaining_line_width', t('widget-editor.remainingThickness', 'Remaining Thickness'), { min: 1, max: 30 })}
          <ColorField
            label={t('widget-editor.finishedColor', 'Finished Color')}
            value={data.completed_line_color}
            onChange={(value) => updateWidgetData(widget.id, { completed_line_color: value })}
          />
          <ColorField
            label={t('widget-editor.remainingColor', 'Remaining Color')}
            value={data.remaining_line_color}
            onChange={(value) => updateWidgetData(widget.id, { remaining_line_color: value })}
          />
          {slider('completed_line_opacity', t('widget-editor.finishedOpacity', 'Finished Opacity'), { min: 0, max: 100, suffix: '%' })}
          {slider('remaining_line_opacity', t('widget-editor.remainingOpacity', 'Remaining Opacity'), { min: 0, max: 100, suffix: '%' })}
        </div>
      </div>

      <div className="space-y-4">
        <SectionHeading icon={Navigation} title={t('widget-editor.routeZoomArrow', 'Arrow')} />
        <div className="grid grid-cols-2 gap-4">
          {slider('marker_size', t('widget-editor.size', 'Size'), { min: 8, max: 120 })}
          {slider('marker_opacity', t('widget-editor.opacity', 'Opacity'), { min: 0, max: 100, suffix: '%' })}
          <ColorField
            label={t('widget-editor.routeZoomArrowColor', 'Arrow Color')}
            value={data.marker_color}
            onChange={(value) => updateWidgetData(widget.id, { marker_color: value })}
          />
          <ColorField
            label={t('widget-editor.routeZoomOutlineColor', 'Outline Color')}
            value={data.marker_outline_color}
            onChange={(value) => updateWidgetData(widget.id, { marker_outline_color: value })}
          />
          {slider('marker_outline_width', t('widget-editor.routeZoomOutlineWidth', 'Outline Thickness'), { min: 0, max: 10 })}
        </div>
      </div>
    </>
  )
}
