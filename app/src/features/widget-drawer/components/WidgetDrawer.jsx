/**
 * Widgets catalog content for the shared left drawer.
 */

import { useLayoutStore, useActivityStore } from '@/hooks/useAppStoreSelectors'
import useStore from '@/store/useStore'
import { useWidgetManager } from '@/features/widget-editor/hooks/useWidgetManager'
import { WidgetButtonGrid } from './WidgetButtonGrid'

/**
 * Provides Widgets content for the shared left drawer.
 * @returns {JSX.Element} Rendered React element.
 */
export function WidgetDrawerContent({ widgetLiveEdits }) {
  const { dismissLeftDrawerOverlay } = useLayoutStore()
  const { activitySummary } = useActivityStore()
  const { addWidget, addWidgetGroup } = useWidgetManager({ widgetLiveEdits })

  const handleAddWidget = (request) => {
    addWidget(request)
    dismissLeftDrawerOverlay()
  }

  const handleAddGroup = async (group) => {
    try {
      await addWidgetGroup(group.templateFile)
      dismissLeftDrawerOverlay()
    } catch (error) {
      console.error('Failed to add widget group:', error)
      useStore.getState().setErrorMessage(`Failed to add ${group.nameDefault}: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  return <WidgetButtonGrid onAddWidget={handleAddWidget} onAddGroup={handleAddGroup} availableMetrics={activitySummary?.availableMetrics ?? []} />
}
