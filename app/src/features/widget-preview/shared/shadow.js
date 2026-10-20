/**
 * Shadow utilities — text shadow and outline shadow computation that mirrors
 * the Skia renderer's shadow behavior.
 */

/**
 * Extracts the text shadow configuration from scene data.
 *
 * Mirrors the Skia renderer's shadow parameter extraction — reads shadow_strength,
 * shadow_distance, and shadow_color from the data object.
 *
 * @param {object|null|undefined} data - Scene or style data with shadow properties.
 * @returns {{ color: string, distance: number, strength: number }|undefined} Shadow config, or undefined if no shadow.
 */
export function getTextShadowParts(data) {
  if (!data?.shadow_color || (data.shadow_strength ?? 0) <= 0) return undefined

  return {
    color: data.shadow_color,
    distance: data.shadow_distance ?? 0,
    strength: data.shadow_strength,
  }
}

/**
 * Applies a widget's optional shadow overrides on top of the scene style.
 *
 * Unset widget values inherit the scene (global) shadow. Returns the original
 * object when nothing is overridden so memoized previews keep a stable identity.
 *
 * @param {object} sceneStyle - Scene style with global shadow settings.
 * @param {object|null|undefined} widgetData - Widget data that may carry shadow_strength / shadow_distance.
 * @returns {object} Scene style to hand to the widget preview.
 */
export function resolveWidgetSceneStyle(sceneStyle, widgetData) {
  const shadowStrength = widgetData?.shadow_strength ?? sceneStyle?.shadow_strength
  const shadowDistance = widgetData?.shadow_distance ?? sceneStyle?.shadow_distance
  if (!sceneStyle || (shadowStrength === sceneStyle.shadow_strength && shadowDistance === sceneStyle.shadow_distance)) return sceneStyle
  return { ...sceneStyle, shadow_strength: shadowStrength, shadow_distance: shadowDistance }
}
