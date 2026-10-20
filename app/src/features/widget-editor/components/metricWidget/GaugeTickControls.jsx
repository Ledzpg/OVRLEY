import { Ruler } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { SectionHeading } from '@/components/ui/section-heading'
import { translateOptions } from '@/i18n'
import { getGaugeScaleDefaults } from '@/lib/widget/standard-metrics'
import { ColorField, NumberField, SelectField, SliderField, ToggleField } from '../widgetFormControls'
import { buildGaugeTickUpdate, MAX_MINOR_TICKS_PER_MAJOR } from './gaugeTickUpdates'

const POSITION_OPTIONS = [
  { value: 'inside', labelKey: 'widget-editor.gaugeTickInside', defaultLabel: 'Inside' },
  { value: 'outside', labelKey: 'widget-editor.gaugeTickOutside', defaultLabel: 'Outside' },
]

/**
 * Dial tick and numeral controls for arc and corner gauges of metrics that support a fixed scale.
 * Ticks need a fixed scale, so the switch stays off until one is set. Renders nothing for other metrics.
 *
 * @param {object} props
 * @param {object} props.widget - Widget being edited.
 * @param {object} props.data - Active display variant data.
 * @param {Function} props.update - Display-variant updater.
 * @returns {JSX.Element|null}
 */
export default function GaugeTickControls({ widget, data, update }) {
  const { t } = useTranslation()
  const defaults = getGaugeScaleDefaults(widget.type)
  if (!defaults) return null

  const fixedScale = data.scale_min != null && data.scale_max != null
  const ticksOn = data.tick_major_interval != null
  const apply = (change) => update(buildGaugeTickUpdate(data, defaults, change))
  const numeric = (field) => (raw) => apply({ field, value: raw })

  return (
    <div className="space-y-4">
      <div className="flex w-full items-center gap-3">
        <SectionHeading icon={Ruler} title={t('widget-editor.gaugeTicks', 'Scale Ticks')} />
        <div className="shrink-0 pt-1">
          <ToggleField
            checked={ticksOn}
            onCheckedChange={(enabled) => apply({ enabled })}
            className={fixedScale ? undefined : 'pointer-events-none opacity-50'}
          />
        </div>
      </div>
      {!fixedScale ? (
        <p className="text-[11px] text-muted-foreground">{t('widget-editor.gaugeTicksNeedScale', 'Turn on Fixed Scale to use ticks.')}</p>
      ) : null}
      {ticksOn ? (
        <>
          <div className="grid grid-cols-2 gap-4">
            <NumberField
              label={t('widget-editor.gaugeTickInterval', 'Major Interval')}
              value={data.tick_major_interval}
              integerDisplay={false}
              onChange={(raw) => apply({ interval: raw })}
            />
            <SliderField
              editable
              label={t('widget-editor.gaugeTickMinorCount', 'Minor Ticks')}
              value={data.tick_minor_count}
              min={0}
              max={MAX_MINOR_TICKS_PER_MAJOR}
              step={1}
              valueDisplay={String(data.tick_minor_count)}
              onSliderChange={(minorCount) => apply({ minorCount })}
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <SelectField
              label={t('widget-editor.gaugeTickPosition', 'Position')}
              value={data.tick_position}
              onValueChange={(position) => apply({ position })}
              options={translateOptions(POSITION_OPTIONS, t)}
            />
            <ColorField
              label={t('widget-editor.gaugeTickColor', 'Tick Color')}
              value={data.tick_color}
              onChange={(tick_color) => update({ tick_color })}
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <NumberField
              label={t('widget-editor.gaugeTickLength', 'Major Length')}
              value={data.tick_major_length}
              integerDisplay={false}
              onChange={numeric('tick_major_length')}
            />
            <NumberField
              label={t('widget-editor.gaugeTickMinorLength', 'Minor Length')}
              value={data.tick_minor_length}
              integerDisplay={false}
              onChange={numeric('tick_minor_length')}
            />
            <NumberField
              label={t('widget-editor.gaugeTickThickness', 'Thickness')}
              value={data.tick_thickness}
              integerDisplay={false}
              onChange={numeric('tick_thickness')}
            />
            <NumberField
              label={t('widget-editor.gaugeTickOffset', 'Distance from Track')}
              value={data.tick_offset}
              integerDisplay={false}
              onChange={numeric('tick_offset')}
            />
          </div>
          <div className="flex w-full items-center gap-3">
            <SectionHeading icon={Ruler} title={t('widget-editor.gaugeTickLabels', 'Numerals')} />
            <div className="shrink-0 pt-1">
              <ToggleField checked={data.tick_show_labels} onCheckedChange={(showLabels) => apply({ showLabels })} />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <NumberField
              label={t('widget-editor.gaugeTickLabelDivisor', 'Divide Numerals By')}
              value={data.tick_label_divisor}
              disabled={!data.tick_show_labels}
              integerDisplay={false}
              onChange={(raw) => apply({ labelDivisor: raw })}
            />
            <NumberField
              label={t('widget-editor.gaugeTickLabelDecimals', 'Decimals')}
              value={data.tick_label_decimals}
              disabled={!data.tick_show_labels}
              onChange={numeric('tick_label_decimals')}
            />
            <NumberField
              label={t('widget-editor.gaugeTickLabelSize', 'Numeral Size')}
              value={data.tick_label_font_size}
              disabled={!data.tick_show_labels}
              integerDisplay={false}
              onChange={numeric('tick_label_font_size')}
            />
            <NumberField
              label={t('widget-editor.gaugeTickLabelOffset', 'Numeral Distance')}
              value={data.tick_label_offset}
              disabled={!data.tick_show_labels}
              integerDisplay={false}
              onChange={numeric('tick_label_offset')}
            />
          </div>
          <ColorField
            label={t('widget-editor.gaugeTickLabelColor', 'Numeral Color')}
            value={data.tick_label_color}
            disabled={!data.tick_show_labels}
            onChange={(tick_label_color) => update({ tick_label_color })}
          />
        </>
      ) : null}
    </div>
  )
}
