/** Fill color band controls: gradient switch, end color and "same as next" layout and behavior. */

import { describe, expect, test, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import GaugeZoneControls from '@/features/widget-editor/components/metricWidget/GaugeZoneControls'

const WIDGET = { id: 'rpm-1', type: 'rpm' }
const BASE = { scale_min: 0, scale_max: 11000, redline_value: 10000, redline_color: '#ff0000' }

function renderControls(zones, update = vi.fn()) {
  render(<GaugeZoneControls widget={WIDGET} data={{ ...BASE, fill_zones: zones }} update={update} />)
  return update
}

const switchNextTo = (label, nth = 0) => screen.getAllByText(label)[nth].parentElement.querySelector('[role="switch"]')

describe('gauge zone controls', () => {
  test('a solid band shows only the gradient switch, centered on the color field', () => {
    renderControls([{ start: 0, color: '#00ff00' }])

    expect(screen.getByText('Gradient to next')).toBeInTheDocument()
    expect(screen.queryByText('End Color')).not.toBeInTheDocument()
    expect(screen.queryByText('Same as next')).not.toBeInTheDocument()
    const row = screen.getByText('Gradient to next').parentElement
    // Same height as the color field and centered, so the switch lines up with it.
    expect(row).toHaveClass('h-9', 'items-center')
    expect(row.parentElement).toHaveClass('items-end')
  })

  test('turning the gradient on follows the next band', async () => {
    const update = renderControls([
      { start: 0, color: '#00ff00' },
      { start: 7000, color: '#ffff00' },
    ])

    await userEvent.setup().click(switchNextTo('Gradient to next'))

    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({ fill_zones: [expect.objectContaining({ end_follows_next: true }), expect.anything()] }),
    )
  })

  test('a gradient shows the end color below with same as next on, and the end color disabled', () => {
    renderControls([
      { start: 0, color: '#00ff00', end_follows_next: true },
      { start: 7000, color: '#ffff00' },
    ])

    expect(screen.getByText('End Color')).toBeInTheDocument()
    expect(switchNextTo('Same as next')).toBeChecked()
    // The end color is the next band's color and cannot be edited while it follows it.
    const endColor = screen.getByText('End Color').parentElement.querySelector('button')
    expect(endColor).toHaveTextContent('#ffff00')
    expect(endColor).toBeDisabled()
    const row = screen.getByText('Same as next').parentElement
    expect(row).toHaveClass('h-9', 'items-center')
  })

  test('the last band follows the redline color', () => {
    renderControls([{ start: 0, color: '#00ff00', end_follows_next: true }])

    expect(screen.getByText('#ff0000')).toBeInTheDocument()
  })

  test('turning same as next off keeps the end color and unlocks it', async () => {
    const update = renderControls([
      { start: 0, color: '#00ff00', end_follows_next: true },
      { start: 7000, color: '#ffff00' },
    ])

    await userEvent.setup().click(switchNextTo('Same as next'))

    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({ fill_zones: [expect.objectContaining({ end_color: '#ffff00', end_follows_next: undefined }), expect.anything()] }),
    )
  })

  test('a gradient with its own end color has same as next off and an editable end color', () => {
    renderControls([
      { start: 0, color: '#00ff00', end_color: '#123456' },
      { start: 7000, color: '#ffff00' },
    ])

    expect(switchNextTo('Same as next')).not.toBeChecked()
    expect(screen.getByText('#123456').closest('button')).not.toBeDisabled()
    expect(screen.getByText('End Color').parentElement.querySelector('button')).toHaveTextContent('#123456')
  })
})
