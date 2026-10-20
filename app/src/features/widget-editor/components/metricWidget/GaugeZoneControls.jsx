import { useState } from 'react'
import { Palette, Plus, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { SectionHeading } from '@/components/ui/section-heading'
import { getGaugeScaleDefaults } from '@/lib/widget/standard-metrics'
import { ColorField, NumberField, ToggleField } from '../widgetFormControls'
import { buildRedlineOverrunUpdate, buildZonesUpdate, MAX_FILL_ZONES, MAX_REDLINE_BLINK_HZ } from './gaugeZoneUpdates'

/**
 * Fill color bands (solid or gradient along the scale) and the full redline fill with optional blinking, for gauges of
 * metrics that support a fixed scale. Bands need a fixed scale; the redline options need a redline.
 * Renders nothing for other metrics.
 *
 * @param {object} props
 * @param {object} props.widget - Widget being edited.
 * @param {object} props.data - Active display variant data.
 * @param {Function} props.update - Display-variant updater.
 * @returns {JSX.Element|null}
 */
export default function GaugeZoneControls({ widget, data, update }) {
  const { t } = useTranslation()
  const defaults = getGaugeScaleDefaults(widget.type)
  const preset = defaults?.zonePreset
  const [peaks, setPeaks] = useState(() => ({ torque: String(preset?.torque ?? ''), power: String(preset?.power ?? '') }))
  if (!defaults) return null

  const fixedScale = data.scale_min != null && data.scale_max != null
  const zones = data.fill_zones ?? []
  const zonesOn = zones.length > 0
  const redlineOn = data.redline_value != null
  const applyZones = (change) => update(buildZonesUpdate(data, defaults, change))
  const applyOverrun = (change) => update(buildRedlineOverrunUpdate(data, change))

  return (
    <div className="space-y-4">
      <div className="flex w-full items-center gap-3">
        <SectionHeading icon={Palette} title={t('widget-editor.gaugeZones', 'Fill Color Bands')} />
        <div className="shrink-0 pt-1">
          <ToggleField
            checked={zonesOn}
            onCheckedChange={(enabled) => applyZones({ enabled })}
            className={fixedScale ? undefined : 'pointer-events-none opacity-50'}
          />
        </div>
      </div>
      {!fixedScale ? (
        <p className="text-[11px] text-muted-foreground">
          {t('widget-editor.gaugeZonesNeedScale', 'Turn on Fixed Scale to color the fill by band.')}
        </p>
      ) : null}
      {zonesOn ? (
        <>
          {preset ? (
            <div className="grid grid-cols-[1fr_1fr_auto] items-end gap-3">
              <NumberField
                label={t('widget-editor.gaugePeakTorque', 'Peak Torque')}
                value={peaks.torque}
                integerDisplay={false}
                onChange={(torque) => setPeaks((current) => ({ ...current, torque }))}
              />
              <NumberField
                label={t('widget-editor.gaugePeakPower', 'Peak Power')}
                value={peaks.power}
                integerDisplay={false}
                onChange={(power) => setPeaks((current) => ({ ...current, power }))}
              />
              <Button variant="secondary" size="sm" onClick={() => applyZones({ preset: peaks })}>
                {t('widget-editor.gaugeZonesFromPeaks', 'Create Bands')}
              </Button>
            </div>
          ) : null}
          {zones.map((zone, index) => (
            <div key={index} className="space-y-2 rounded-md border border-border/60 p-3">
              <div className="grid grid-cols-[1fr_auto] items-end gap-3">
                <NumberField
                  label={index === 0 ? t('widget-editor.gaugeZoneFirst', 'From (scale minimum)') : t('widget-editor.gaugeZoneStart', 'From')}
                  value={zone.start}
                  disabled={index === 0}
                  integerDisplay={false}
                  onChange={(start) => applyZones({ index, start })}
                />
                {index > 0 ? (
                  <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => applyZones({ index, remove: true })}>
                    <X className="h-3.5 w-3.5" />
                  </Button>
                ) : null}
              </div>
              <div className="grid grid-cols-2 gap-4">
                <ColorField
                  label={t('widget-editor.gaugeZoneColor', 'Color')}
                  value={zone.color}
                  onChange={(color) => applyZones({ index, color })}
                />
                <div className="space-y-1">
                  <div className="flex items-center gap-3">
                    <span className="text-[9px] font-bold uppercase text-muted-foreground">
                      {t('widget-editor.gaugeZoneGradient', 'Gradient to next')}
                    </span>
                    <ToggleField checked={zone.end_color != null} onCheckedChange={(gradient) => applyZones({ index, gradient })} />
                  </div>
                  {zone.end_color != null ? (
                    <ColorField
                      label={t('widget-editor.gaugeZoneEndColor', 'End Color')}
                      value={zone.end_color}
                      onChange={(endColor) => applyZones({ index, endColor })}
                    />
                  ) : null}
                </div>
              </div>
            </div>
          ))}
          {zones.length < MAX_FILL_ZONES ? (
            <Button variant="secondary" size="sm" onClick={() => applyZones({ add: true })}>
              <Plus className="mr-1 h-3.5 w-3.5" />
              {t('widget-editor.gaugeZoneAdd', 'Add Band')}
            </Button>
          ) : null}
        </>
      ) : null}

      {redlineOn ? (
        <>
          <div className="flex w-full items-center gap-3">
            <SectionHeading icon={Palette} title={t('widget-editor.redlineFillAll', 'Fill Completely at Redline')} />
            <div className="shrink-0 pt-1">
              <ToggleField checked={data.redline_fill_all === true} onCheckedChange={(fillAll) => applyOverrun({ fillAll })} />
            </div>
          </div>
          {data.redline_fill_all ? (
            <div className="grid grid-cols-2 items-end gap-4">
              <div className="flex items-center gap-3 pb-2">
                <span className="text-[9px] font-bold uppercase text-muted-foreground">{t('widget-editor.redlineBlink', 'Blink')}</span>
                <ToggleField checked={data.redline_blink_hz != null} onCheckedChange={(blink) => applyOverrun({ blink })} />
              </div>
              <NumberField
                label={t('widget-editor.redlineBlinkRate', 'Flashes per Second')}
                value={data.redline_blink_hz}
                disabled={data.redline_blink_hz == null}
                min={0.5}
                max={MAX_REDLINE_BLINK_HZ}
                integerDisplay={false}
                onChange={(blinkHz) => applyOverrun({ blinkHz })}
              />
            </div>
          ) : null}
        </>
      ) : null}
    </div>
  )
}
