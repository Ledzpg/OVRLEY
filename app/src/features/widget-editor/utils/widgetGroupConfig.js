/** Pure helpers for adding a group of widgets (taken from a template) to the current config. */

import { ensureWidgetIdsInConfig } from '@/lib/widget/widget-config'

const GROUP_CATEGORIES = ['backdrops', 'labels', 'values', 'plots']

/**
 * Appends every widget of a template config to the current config, giving them fresh ids.
 *
 * Template ids are dropped so they cannot collide with (or be mistaken for) widgets already in the editor.
 * Rasters are not part of groups and are left alone.
 *
 * @param {object} config - Current editor config.
 * @param {object} groupConfig - Config of the template that defines the group.
 * @returns {{ config: object, addedIds: string[] }} The merged config and the ids of the added widgets in
 *   backdrop, label, value, plot order.
 */
export function appendWidgetGroup(config, groupConfig) {
  const appended = {}
  for (const category of GROUP_CATEGORIES) {
    appended[category] = groupConfig[category].map(({ id: _templateId, ...widget }) => widget)
  }
  const merged = ensureWidgetIdsInConfig({
    ...config,
    ...Object.fromEntries(GROUP_CATEGORIES.map((category) => [category, [...config[category], ...appended[category]]])),
  })
  const addedIds = GROUP_CATEGORIES.flatMap((category) => merged[category].slice(config[category].length).map((widget) => widget.id))
  return { config: merged, addedIds }
}
