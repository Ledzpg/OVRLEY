/**
 * Widget groups offered in the widget drawer: several widgets added together, laid out as in the bundled template
 * of the same file. The template file is the single source of the layout. Each group is listed in the widget
 * category named by `category`, after the single widgets of that category.
 */

import { Gauge, Thermometer } from 'lucide-react'

export const WIDGET_GROUPS = Object.freeze([
  {
    id: 'tach-cluster',
    category: 'motosports',
    templateFile: 'cluster-tacometro-velocimetro.json',
    icon: Gauge,
    shortNameKey: 'widget-drawer.groupTachClusterShort',
    shortNameDefault: 'Cluster',
    nameKey: 'widget-drawer.groupTachCluster',
    nameDefault: 'Tachometer + speed + gear',
  },
  {
    id: 'temperature-panel',
    category: 'motosports',
    templateFile: 'painel-temperaturas.json',
    icon: Thermometer,
    shortNameKey: 'widget-drawer.groupTemperaturesShort',
    shortNameDefault: 'Temps.',
    nameKey: 'widget-drawer.groupTemperatures',
    nameDefault: 'Coolant + intake air temperatures',
  },
])
