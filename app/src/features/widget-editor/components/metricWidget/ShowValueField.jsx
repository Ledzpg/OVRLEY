import { useTranslation } from 'react-i18next'
import { ToggleField } from '../widgetFormControls'

/**
 * Toggle for drawing the number of a metric widget. Turning it off leaves only the icon,
 * the unit, or (for gauges) the bars.
 */
export default function ShowValueField({ widget, updateWidgetData }) {
  const { t } = useTranslation()
  return (
    <div className="flex items-center justify-between py-2">
      <span className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">{t('widget-editor.showValue', 'Show Value')}</span>
      <ToggleField checked={widget.data.show_value} onCheckedChange={(show_value) => updateWidgetData(widget.id, { show_value })} />
    </div>
  )
}
