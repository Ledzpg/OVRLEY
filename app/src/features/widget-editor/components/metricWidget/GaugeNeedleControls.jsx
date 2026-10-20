import { Navigation } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { SectionHeading } from '@/components/ui/section-heading'
import { ColorField, NumberField, SliderField, ToggleField } from '../widgetFormControls'
import { buildNeedleUpdate } from './gaugeNeedleUpdates'

/**
 * Needle controls for arc and corner gauges: a short bar across the track that follows the value.
 *
 * @param {object} props
 * @param {object} props.data - Active display variant data.
 * @param {Function} props.update - Display-variant updater.
 * @returns {JSX.Element}
 */
export default function GaugeNeedleControls({ data, update }) {
  const { t } = useTranslation()
  const needleOn = data.needle_length != null
  const gradientOn = needleOn && data.needle_fill_end_color != null
  const apply = (change) => update(buildNeedleUpdate(data, change))
  const numeric = (field) => (value) => apply({ field, value })

  return (
    <div className="space-y-4">
      <div className="flex w-full items-center gap-3">
        <SectionHeading icon={Navigation} title={t('widget-editor.gaugeNeedle', 'Needle')} />
        <div className="shrink-0 pt-1">
          <ToggleField checked={needleOn} onCheckedChange={(enabled) => apply({ enabled })} />
        </div>
      </div>
      {needleOn ? (
        <>
          <div className="grid grid-cols-2 gap-4">
            <NumberField
              label={t('widget-editor.gaugeNeedleLength', 'Length')}
              value={data.needle_length}
              integerDisplay={false}
              onChange={numeric('needle_length')}
            />
            <NumberField
              label={t('widget-editor.gaugeNeedleWidth', 'Width')}
              value={data.needle_width}
              integerDisplay={false}
              onChange={numeric('needle_width')}
            />
            <NumberField
              label={t('widget-editor.gaugeNeedleOffset', 'Offset from Track')}
              value={data.needle_offset}
              integerDisplay={false}
              onChange={numeric('needle_offset')}
            />
            <NumberField
              label={t('widget-editor.gaugeNeedleCorner', 'Corner Radius')}
              value={data.needle_corner_radius}
              integerDisplay={false}
              onChange={numeric('needle_corner_radius')}
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <ColorField
              label={t('widget-editor.gaugeNeedleColor', 'Border Color')}
              value={data.needle_color}
              onChange={(color) => apply({ color })}
            />
            <NumberField
              label={t('widget-editor.gaugeNeedleBorder', 'Border Thickness')}
              value={data.needle_border_thickness}
              integerDisplay={false}
              onChange={numeric('needle_border_thickness')}
            />
          </div>
          <div className="flex w-full items-center gap-3">
            <SectionHeading icon={Navigation} title={t('widget-editor.gaugeNeedleGradient', 'Gradient Fill')} />
            <div className="shrink-0 pt-1">
              <ToggleField checked={gradientOn} onCheckedChange={(gradient) => apply({ gradient })} />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <ColorField
              label={gradientOn ? t('widget-editor.gaugeNeedleStartColor', 'Start Color') : t('widget-editor.gaugeNeedleFillColor', 'Fill Color')}
              value={data.needle_fill_color ?? data.needle_color}
              onChange={(fillColor) => apply({ fillColor })}
            />
            <SliderField
              editable
              label={gradientOn ? t('widget-editor.gaugeNeedleStartOpacity', 'Start Opacity') : t('widget-editor.gaugeNeedleFill', 'Fill Opacity')}
              value={Math.round(data.needle_fill_opacity * 100)}
              min={0}
              max={100}
              step={1}
              valueDisplay={`${Math.round(data.needle_fill_opacity * 100)}`}
              suffix="%"
              onSliderChange={(fillOpacity) => apply({ fillOpacity })}
            />
          </div>
          {gradientOn ? (
            <div className="grid grid-cols-2 gap-4">
              <ColorField
                label={t('widget-editor.gaugeNeedleEndColor', 'End Color')}
                value={data.needle_fill_end_color}
                onChange={(fillEndColor) => apply({ fillEndColor })}
              />
              <SliderField
                editable
                label={t('widget-editor.gaugeNeedleEndOpacity', 'End Opacity')}
                value={Math.round(data.needle_fill_end_opacity * 100)}
                min={0}
                max={100}
                step={1}
                valueDisplay={`${Math.round(data.needle_fill_end_opacity * 100)}`}
                suffix="%"
                onSliderChange={(fillEndOpacity) => apply({ fillEndOpacity })}
              />
            </div>
          ) : null}
        </>
      ) : null}
    </div>
  )
}
