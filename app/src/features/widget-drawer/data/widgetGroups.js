/**
 * Widget groups offered in the widget drawer: several widgets added together, laid out as in the bundled template
 * of the same file. The template file is the single source of the layout.
 */

import { Gauge, Thermometer } from 'lucide-react'

export const WIDGET_GROUPS = Object.freeze([
  {
    id: 'tach-cluster',
    templateFile: 'cluster-tacometro-velocimetro.json',
    icon: Gauge,
    shortNameKey: 'widget-drawer.groupTachClusterShort',
    shortNameDefault: 'Cluster',
    nameKey: 'widget-drawer.groupTachCluster',
    nameDefault: 'Tachometer + speed + gear',
  },
  {
    id: 'temperature-panel',
    templateFile: 'painel-temperaturas.json',
    icon: Thermometer,
    shortNameKey: 'widget-drawer.groupTemperaturesShort',
    shortNameDefault: 'Temps.',
    nameKey: 'widget-drawer.groupTemperatures',
    nameDefault: 'Coolant + intake air temperatures',
  },
])
