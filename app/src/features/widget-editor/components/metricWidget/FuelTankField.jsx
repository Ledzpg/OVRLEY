import { useTranslation } from 'react-i18next'
import { NumberField } from '../widgetFormControls'
import { buildFuelTankUpdate, FUEL_TANK_MAX } from './fuelTankUpdates'

/**
 * Tank capacity of a fuel level widget, used to show the level as a percentage.
 *
 * @param {object} props
 * @param {object} props.widget - Fuel level widget config.
 * @param {Function} props.updateWidgetData - Updates widget data.
 */
export default function FuelTankField({ widget, updateWidgetData }) {
  const { t } = useTranslation()

  return (
    <div className="grid grid-cols-2 gap-4">
      <NumberField
        label={t('widget-editor.fuelTankCapacity', 'Tank Capacity')}
        value={widget.data.fuel_tank_capacity}
        min={1}
        max={FUEL_TANK_MAX}
        step={0.5}
        suffix="L"
        integerDisplay={false}
        onChange={(value) => updateWidgetData(widget.id, buildFuelTankUpdate(value))}
      />
    </div>
  )
}
