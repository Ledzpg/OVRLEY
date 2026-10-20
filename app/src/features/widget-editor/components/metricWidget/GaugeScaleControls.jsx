import { Gauge } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { SectionHeading } from '@/components/ui/section-heading'
import { getGaugeScaleDefaults } from '@/lib/widget/standard-metrics'
import { ColorField, NumberField, ToggleField } from '../widgetFormControls'
import { buildGaugeScaleUpdate, DEFAULT_REDLINE_COLOR } from './gaugeScaleUpdates'

/**
 * Fixed scale and redline controls for gauges of metrics that support them (tachometer, temperatures).
 * Renders nothing for other metrics. Values use the metric's canonical unit (rpm, °C).
 *
 * @param {object} props
 * @param {object} props.widget - Widget being edited.
 * @param {object} props.data - Active display variant data.
 * @param {Function} props.update - Display-variant updater.
 * @returns {JSX.Element|null}
 */
export default function GaugeScaleControls({ widget, data, update }) {
  const { t } = useTranslation()
  const defaults = getGaugeScaleDefaults(widget.type)
  if (!defaults) return null

  const fixedScale = data.scale_min != null && data.scale_max != null
  const redlineOn = data.redline_value != null

  return (
    <div className="space-y-4">
      <div className="flex w-full items-center gap-3">
        <SectionHeading icon={Gauge} title={t('widget-editor.fixedScale', 'Fixed Scale')} />
        <div className="shrink-0 pt-1">
          <ToggleField checked={fixedScale} onCheckedChange={(enabled) => update(buildGaugeScaleUpdate(data, defaults, { fixedScale: enabled }))} />
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <NumberField
          label={t('widget-editor.scaleMin', 'Minimum')}
          value={data.scale_min}
          disabled={!fixedScale}
          integerDisplay={false}
          onChange={(raw) => update(buildGaugeScaleUpdate(data, defaults, { scaleMin: raw }))}
        />
        <NumberField
          label={t('widget-editor.scaleMax', 'Maximum')}
          value={data.scale_max}
          disabled={!fixedScale}
          integerDisplay={false}
          onChange={(raw) => update(buildGaugeScaleUpdate(data, defaults, { scaleMax: raw }))}
        />
      </div>
      <div className="flex w-full items-center gap-3">
        <SectionHeading icon={Gauge} title={t('widget-editor.redline', 'Redline')} />
        <div className="shrink-0 pt-1">
          <ToggleField checked={redlineOn} onCheckedChange={(enabled) => update(buildGaugeScaleUpdate(data, defaults, { redline: enabled }))} />
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <NumberField
          label={t('widget-editor.redlineStart', 'Starts at')}
          value={data.redline_value}
          disabled={!redlineOn}
          integerDisplay={false}
          onChange={(raw) => update(buildGaugeScaleUpdate(data, defaults, { redlineValue: raw }))}
        />
        <ColorField
          label={t('widget-editor.redlineColor', 'Redline Color')}
          value={data.redline_color ?? DEFAULT_REDLINE_COLOR}
          disabled={!redlineOn}
          onChange={(redline_color) => update({ redline_color })}
        />
      </div>
    </div>
  )
}
