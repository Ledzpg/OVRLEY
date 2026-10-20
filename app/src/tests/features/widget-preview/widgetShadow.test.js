import { describe, expect, test } from 'vitest'
import { getTextShadowParts, resolveWidgetSceneStyle } from '@/features/widget-preview/shared/shadow'
import { getEffectiveWidgetData } from '@/lib/template/template-state'

const sceneStyle = { shadow_color: '#000000', shadow_strength: 4, shadow_distance: 3 }

describe('resolveWidgetSceneStyle', () => {
  test('inherits the global shadow when the widget sets nothing', () => {
    expect(resolveWidgetSceneStyle(sceneStyle, {})).toBe(sceneStyle)
  })

  test('applies widget strength and distance independently', () => {
    expect(resolveWidgetSceneStyle(sceneStyle, { shadow_strength: 9 })).toEqual({ ...sceneStyle, shadow_strength: 9 })
    expect(resolveWidgetSceneStyle(sceneStyle, { shadow_distance: 0 })).toEqual({ ...sceneStyle, shadow_distance: 0 })
  })

  test('a widget strength of 0 disables the shadow for that widget only', () => {
    const resolved = resolveWidgetSceneStyle(sceneStyle, { shadow_strength: 0 })
    expect(getTextShadowParts(resolved)).toBeUndefined()
    expect(getTextShadowParts(sceneStyle)).toBeDefined()
  })
})

describe('per-widget shadow materialization', () => {
  const globals = { shadow_strength: 4, shadow_distance: 3, opacity: 1, font_text: 'Arial.ttf', font_values: 'Arial.ttf' }

  test.each(['labels', 'values', 'plots'])('%s inherit global shadow values when unset', (category) => {
    const data = getEffectiveWidgetData({ category, data: { value: 'speed', display_type: 'text' } }, globals)
    expect(data.shadow_strength).toBe(4)
    expect(data.shadow_distance).toBe(3)
  })

  test.each(['labels', 'values', 'plots'])('%s keep their own shadow values', (category) => {
    const data = getEffectiveWidgetData({ category, data: { value: 'speed', display_type: 'text', shadow_strength: 0, shadow_distance: 8 } }, globals)
    expect(data.shadow_strength).toBe(0)
    expect(data.shadow_distance).toBe(8)
  })
})
