/**
 * Tests for widget groups in the drawer: several widgets added together from a bundled template.
 */

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cwd } from 'node:process'
import { beforeEach, describe, expect, test, vi } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { WidgetButtonGrid } from '@/features/widget-drawer/components/WidgetButtonGrid'
import { WidgetDrawerContent as WidgetDrawerView } from '@/features/widget-drawer'
import { WIDGET_GROUPS } from '@/features/widget-drawer/data/widgetGroups'
import { appendWidgetGroup } from '@/features/widget-editor/utils/widgetGroupConfig'
import useWidgetDraftState from '@/features/overlay-editor/hooks/useWidgetDraftState'
import { normalizeTemplateFilePayload } from '@/features/template-manager/utils/templateSnapshot'
import { cloneSerializable, DEFAULT_CONFIG } from '@/store/store-utils'
import useStore from '@/store/useStore'
import * as backend from '@/api/backend'

// Vitest runs from app/ (see agents.md), so the bundled templates live one level up.
const readBundledTemplate = (file) => readFileSync(resolve(cwd(), '..', 'templates', file), 'utf8')

vi.mock('@/api/backend', async (importOriginal) => ({ ...(await importOriginal()), getTemplate: vi.fn() }))
vi.mock('@/features/template-manager/utils/templateSnapshot', async (importOriginal) => {
  const original = await importOriginal()
  // Fonts and rasters need the app runtime; the layout normalization is the real one.
  return { ...original, prepareTemplateFilePayload: async (raw) => ({ templateState: original.normalizeTemplateFilePayload(JSON.parse(raw)) }) }
})

function WidgetDrawerContent() {
  const widgetLiveEdits = useWidgetDraftState()
  return <WidgetDrawerView widgetLiveEdits={widgetLiveEdits} />
}

const count = (config) => config.backdrops.length + config.labels.length + config.values.length + config.plots.length

beforeEach(() => {
  useStore.setState({
    config: cloneSerializable(DEFAULT_CONFIG),
    leftDrawerPinned: false,
    leftDrawerVisible: true,
    selectedWidgetId: null,
    selectedWidgetIds: [],
    errorMessage: null,
  })
  backend.getTemplate.mockReset()
})

describe('widget group catalog', () => {
  test.each(WIDGET_GROUPS)('$id points at a bundled template with widgets', (group) => {
    const state = normalizeTemplateFilePayload(JSON.parse(readBundledTemplate(group.templateFile)))
    expect(count(state.config)).toBeGreaterThan(1)
  })
})

describe('appendWidgetGroup', () => {
  const group = {
    backdrops: [{ id: 'widget-1', x: 0 }],
    labels: [{ id: 'widget-2', text: 'A' }],
    values: [{ id: 'widget-3', value: 'rpm' }],
    plots: [],
  }

  test('keeps the existing widgets, appends the group with fresh unique ids and reports them in order', () => {
    const config = { backdrops: [{ id: 'widget-1', x: 5 }], labels: [], values: [{ id: 'widget-2', value: 'speed' }], plots: [], rasters: [] }
    const { config: merged, addedIds } = appendWidgetGroup(config, group)

    expect(merged.backdrops[0]).toEqual({ id: 'widget-1', x: 5 })
    expect(merged.values[0]).toEqual({ id: 'widget-2', value: 'speed' })
    expect(merged.backdrops).toHaveLength(2)
    expect(merged.labels).toHaveLength(1)
    expect(merged.values).toHaveLength(2)
    expect(addedIds).toHaveLength(3)
    const all = [...merged.backdrops, ...merged.labels, ...merged.values].map((widget) => widget.id)
    expect(new Set(all).size).toBe(all.length)
    expect(addedIds).toEqual([merged.backdrops[1].id, merged.labels[0].id, merged.values[1].id])
  })

  test('does not mutate the current config or the group', () => {
    const config = { backdrops: [], labels: [], values: [], plots: [], rasters: [] }
    const before = JSON.stringify(config)
    const groupBefore = JSON.stringify(group)
    appendWidgetGroup(config, group)
    expect(JSON.stringify(config)).toBe(before)
    expect(JSON.stringify(group)).toBe(groupBefore)
  })
})

describe('widget group buttons', () => {
  test('the grid lists every group and reports the clicked one', async () => {
    const onAddGroup = vi.fn()
    const user = userEvent.setup()
    render(<WidgetButtonGrid onAddWidget={() => {}} onAddGroup={onAddGroup} availableMetrics={[]} />)

    expect(screen.queryByText('Sets')).not.toBeInTheDocument()
    const motorsports = screen.getByText('Motorsports').parentElement
    for (const group of WIDGET_GROUPS) {
      expect(within(motorsports).getByRole('button', { name: group.nameDefault })).toBeInTheDocument()
    }
    await user.click(screen.getByRole('button', { name: 'Tachometer + speed + gear' }))
    expect(onAddGroup).toHaveBeenCalledWith(WIDGET_GROUPS[0])
    expect(screen.getByRole('button', { name: 'Coolant + intake air temperatures' })).toBeInTheDocument()
  })
})

describe('adding a group from the drawer', () => {
  test.each(WIDGET_GROUPS)('adds every widget of $id together, selects the first and closes the drawer', async (group) => {
    const user = userEvent.setup()
    const raw = readBundledTemplate(group.templateFile)
    backend.getTemplate.mockResolvedValue(raw)
    const expected = count(normalizeTemplateFilePayload(JSON.parse(raw)).config)
    render(<WidgetDrawerContent />)

    await user.click(screen.getByRole('button', { name: group.nameDefault }))

    await waitFor(() => expect(useStore.getState().leftDrawerVisible).toBe(false))
    expect(backend.getTemplate).toHaveBeenCalledWith(group.templateFile)
    const { config, selectedWidgetId } = useStore.getState()
    expect(count(config)).toBe(expected)
    const ids = [...config.backdrops, ...config.labels, ...config.values, ...config.plots].map((widget) => widget.id)
    expect(new Set(ids).size).toBe(ids.length)
    expect(ids).toContain(selectedWidgetId)
  })

  test('adding twice keeps both copies with distinct ids', async () => {
    const user = userEvent.setup()
    backend.getTemplate.mockResolvedValue(readBundledTemplate(WIDGET_GROUPS[1].templateFile))
    useStore.setState({ leftDrawerPinned: true })
    render(<WidgetDrawerContent />)

    await user.click(screen.getByRole('button', { name: WIDGET_GROUPS[1].nameDefault }))
    await waitFor(() => expect(useStore.getState().config.values.length).toBeGreaterThan(0))
    const afterFirst = count(useStore.getState().config)
    await user.click(screen.getByRole('button', { name: WIDGET_GROUPS[1].nameDefault }))
    await waitFor(() => expect(count(useStore.getState().config)).toBe(afterFirst * 2))
    const { config } = useStore.getState()
    const ids = [...config.backdrops, ...config.labels, ...config.values].map((widget) => widget.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  test('a template that cannot be read reports the error and changes nothing', async () => {
    const user = userEvent.setup()
    backend.getTemplate.mockRejectedValue(new Error('Template not found'))
    vi.spyOn(console, 'error').mockImplementation(() => {})
    render(<WidgetDrawerContent />)

    await user.click(screen.getByRole('button', { name: WIDGET_GROUPS[0].nameDefault }))

    await waitFor(() => expect(useStore.getState().errorMessage).toContain('Template not found'))
    expect(count(useStore.getState().config)).toBe(0)
    expect(useStore.getState().leftDrawerVisible).toBe(true)
  })
})
