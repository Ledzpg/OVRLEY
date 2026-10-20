/**
 * Fuel level, battery voltage and estimated range: catalog entries, unit conversion, defaults and the tank
 * capacity control.
 */

import { describe, expect, test, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import FuelTankField from '@/features/widget-editor/components/metricWidget/FuelTankField'
import { buildFuelTankUpdate, FUEL_TANK_MAX } from '@/features/widget-editor/components/metricWidget/fuelTankUpdates'
import { convertStandardMetricValue, getStandardMetricDefinition, getStandardMetricUnitOptions } from '@/lib/widget/standard-metrics'
import { TYPE_DEFAULTS, WIDGET_TYPE_DEFINITIONS } from '@/lib/widget/standard-widgets'
import { WIDGET_ICON_SVGS } from '@/lib/widget/widget-icons'
import { formatGaugeBoundaryLabel } from '@/features/widget-preview/shared/gaugeLabelFormat'

describe('catalog entries', () => {
  test.each([
    ['fuel_level', 'widget-fuel.svg', ['l', 'percent']],
    ['battery_voltage', 'widget-battery.svg', ['volt']],
    ['estimated_range', 'widget-range.svg', ['km', 'mi']],
  ])('%s is a motorsports widget with its own icon and units', (type, icon, units) => {
    expect(WIDGET_TYPE_DEFINITIONS[type]).toMatchObject({ category: 'motosports' })
    expect(getStandardMetricDefinition(type).icon.assetFile).toBe(icon)
    expect(getStandardMetricUnitOptions(type).map((option) => option.value)).toEqual(units)
    expect(WIDGET_ICON_SVGS[type].innerMarkup).not.toBe('')
  })

  test('a new fuel level widget starts with litres and a tank capacity', () => {
    expect(TYPE_DEFAULTS.fuel_level).toMatchObject({ display_unit: 'l', fuel_tank_capacity: 21, decimals: 1 })
    expect(TYPE_DEFAULTS.battery_voltage).not.toHaveProperty('fuel_tank_capacity')
  })
})

describe('unit conversion', () => {
  test('fuel level is shown in litres or as a percentage of the tank', () => {
    expect(convertStandardMetricValue('fuel_level', 10.5, 'l', 21)).toBe(10.5)
    expect(convertStandardMetricValue('fuel_level', 10.5, 'percent', 21)).toBe(50)
    expect(convertStandardMetricValue('fuel_level', 10.5, 'percent', 14)).toBe(75)
  })

  test('estimated range is stored in metres and shown in kilometres or miles', () => {
    expect(convertStandardMetricValue('estimated_range', 160934.4, 'km')).toBeCloseTo(160.9344)
    expect(convertStandardMetricValue('estimated_range', 160934.4, 'mi')).toBeCloseTo(100)
  })

  test('gauge boundary labels follow the unit and the tank capacity', () => {
    expect(formatGaugeBoundaryLabel('fuel_level', 21, 'percent', 21)).toBe('100')
    expect(formatGaugeBoundaryLabel('fuel_level', 21, 'l', 21)).toBe('21')
    expect(formatGaugeBoundaryLabel('estimated_range', 400000, 'mi')).toBe('249')
  })
})

describe('tank capacity control', () => {
  test('accepts positive litres up to the maximum and ignores anything else', () => {
    expect(buildFuelTankUpdate('17.5')).toEqual({ fuel_tank_capacity: 17.5 })
    expect(buildFuelTankUpdate(FUEL_TANK_MAX)).toEqual({ fuel_tank_capacity: FUEL_TANK_MAX })
    for (const invalid of ['', null, undefined, 'abc', 0, -3, FUEL_TANK_MAX + 1]) {
      expect(buildFuelTankUpdate(invalid)).toEqual({})
    }
  })

  test('shows the capacity of the widget and writes a valid change', async () => {
    const updateWidgetData = vi.fn()
    const user = userEvent.setup()
    render(<FuelTankField widget={{ id: 'fuel-1', type: 'fuel_level', data: { fuel_tank_capacity: 21 } }} updateWidgetData={updateWidgetData} />)

    const input = screen.getByRole('spinbutton')
    expect(input).toHaveValue(21)

    await user.clear(input)
    await user.type(input, '18')
    await user.tab()

    expect(updateWidgetData).toHaveBeenLastCalledWith('fuel-1', { fuel_tank_capacity: 18 })
  })
})
