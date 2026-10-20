/**
 * Controls for the raster tile map drawn behind the route widget.
 */

import { Globe } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Label } from '@/components/ui/label'
import { SectionHeading } from '@/components/ui/section-heading'
import { translateOptions } from '@/i18n'
import { SelectField, SliderField, TextField, ToggleField } from './widgetFormControls'
import { buildMapUpdate, CUSTOM_MAP_PRESET, getMapPresetId, MAP_MAX_ZOOM_OFFSET, MAP_MIN_OPACITY } from './routeMapUpdates'

const PROVIDER_OPTIONS = [
  { value: 'osm', labelKey: 'widget-editor.routeMapProviderOsm', defaultLabel: 'OpenStreetMap' },
  { value: 'opentopomap', labelKey: 'widget-editor.routeMapProviderTopo', defaultLabel: 'OpenTopoMap' },
  { value: 'esri-light-gray', labelKey: 'widget-editor.routeMapProviderLightGray', defaultLabel: 'Light gray (Esri)' },
  { value: CUSTOM_MAP_PRESET, labelKey: 'widget-editor.routeMapProviderCustom', defaultLabel: 'Custom URL' },
]

/**
 * Renders the map background section of the route widget editor.
 *
 * @param {object} props - Component props.
 * @param {object} props.widget - Route widget being edited.
 * @param {Function} props.updateWidgetData - Merges a patch into the widget data.
 * @param {boolean} [props.alwaysOn=false] - The map is part of the widget (close-up map): no on/off switch.
 * @returns {JSX.Element} Rendered component output.
 */
export default function RouteMapBackgroundSection({ widget, updateWidgetData, alwaysOn = false }) {
  const { t } = useTranslation()
  const providerOptions = useMemo(() => translateOptions(PROVIDER_OPTIONS, t), [t])
  const data = widget.data
  const enabled = data.map_tile_url != null
  const apply = (change) => updateWidgetData(widget.id, buildMapUpdate(data, change))

  return (
    <div className="space-y-4">
      <SectionHeading icon={Globe} title={t('widget-editor.routeMapBackground', 'Map Background')} />
      {alwaysOn ? null : (
        <div className="flex items-center justify-between gap-2 pl-1 pt-2 pb-2">
          <Label className="p-0 text-[9px] text-muted-foreground uppercase font-bold">{t('widget-editor.routeMapEnabled', 'Show Map')}</Label>
          <ToggleField checked={enabled} onCheckedChange={(checked) => apply({ enabled: checked })} />
        </div>
      )}
      {enabled ? (
        <>
          <SelectField
            label={t('widget-editor.routeMapProvider', 'Map Style')}
            value={getMapPresetId(data)}
            options={providerOptions}
            onValueChange={(value) => (value === CUSTOM_MAP_PRESET ? undefined : apply({ preset: value }))}
          />
          {getMapPresetId(data) === CUSTOM_MAP_PRESET ? (
            <TextField
              label={t('widget-editor.routeMapTileUrl', 'Tile URL')}
              value={data.map_tile_url}
              onChange={(value) => apply({ tileUrl: value })}
            />
          ) : null}
          <TextField
            label={t('widget-editor.routeMapAttribution', 'Credit')}
            value={data.map_attribution}
            onChange={(value) => apply({ attribution: value })}
          />
          <div className="grid grid-cols-2 gap-4">
            <SliderField
              editable
              label={t('widget-editor.routeMapOpacity', 'Map Opacity')}
              value={data.map_opacity}
              min={MAP_MIN_OPACITY}
              max={100}
              step={1}
              valueDisplay={`${data.map_opacity}`}
              suffix="%"
              onSliderChange={(value) => apply({ opacity: value })}
            />
            <SliderField
              editable
              label={t('widget-editor.routeMapZoomOffset', 'Map Detail')}
              value={data.map_zoom_offset}
              min={-MAP_MAX_ZOOM_OFFSET}
              max={MAP_MAX_ZOOM_OFFSET}
              step={1}
              integerDisplay
              valueDisplay={`${data.map_zoom_offset}`}
              onSliderChange={(value) => apply({ zoomOffset: value })}
            />
          </div>
          <p className="text-[10px] text-muted-foreground">{t('widget-editor.routeMapNeedsInternet', 'Tiles are downloaded once and cached.')}</p>
        </>
      ) : null}
    </div>
  )
}
