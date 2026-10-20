/** Pure update builder for the fuel tank capacity control. */

/** Largest tank capacity (litres) the renderer accepts. */
export const FUEL_TANK_MAX = 1000

/**
 * Builds the widget-data patch for one change to the tank capacity.
 *
 * Invalid input (empty, non-numeric, zero or negative, above the maximum) yields an empty patch so the config
 * never reaches the renderer in a state it would reject.
 *
 * @param {string|number} raw - Value typed by the user, in litres.
 * @returns {{ fuel_tank_capacity?: number }} Widget-data patch.
 */
export function buildFuelTankUpdate(raw) {
  if (raw === '' || raw == null) return {}
  const capacity = Number(raw)
  if (!Number.isFinite(capacity) || capacity <= 0 || capacity > FUEL_TANK_MAX) return {}
  return { fuel_tank_capacity: capacity }
}
