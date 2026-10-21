# Importing vehicle telemetry (CSV)

OVRLEY reads a plain CSV file and draws the columns as widgets over your video: speed, RPM, gear, throttle, engine
load, G-force, temperatures, fuel level, battery voltage, estimated range, heading, and the route. This page describes
the layout this fork documents and tests, and the template you can start from.

- **Template:** [`telemetry-template.csv`](telemetry-template.csv): 25 samples with every column filled in
- Load it like any activity file (the import button, or drag and drop). It also shows what each widget does with real
  numbers, so it is a quick way to try a template.

## How it works

1. You export your logger's data as a CSV, or write one yourself, with a **header row** and **one row per sample**.
2. On import, OVRLEY reads the header and recognizes each column by name (see
   [How columns are recognized](#how-columns-are-recognized)). Columns it does not know are ignored.
3. Values are converted to one internal unit per metric (km/h to m/s, °F to °C, miles to metres, and so on), invalid
   readings are dropped, and the samples are lined up on the video timeline.
4. Each widget draws the metric it is set to. A widget whose column is missing from the file is shown as "no data" and
   draws nothing.

Only columns you actually have are needed. A file with just a time column and `Speed (km/h)` works.

## The file

| Rule | Detail |
| --- | --- |
| Format | Comma-separated, UTF-8, first row is the header. |
| Samples | At least **two** rows with a time, in ascending time order. Any rate works; 10 samples per second gives smooth gauges. |
| Empty cell | A gap: "no reading" for that sample. Do not write `0` for a sensor that did not answer. |
| Decimal separator | A dot (`.`). |
| Units | Written in the header, in parentheses: `Speed (km/h)`. Without a unit, the metric's default unit is used (see the table). |

## Columns of the template

The template uses these 18 columns, in this order. The order does not matter to OVRLEY; the names do.

| Column | Unit | What it is | Default if no unit is written |
| --- | --- | --- | --- |
| `DateTime (UTC)` | ISO 8601, e.g. `2026-01-01T12:00:00.000Z` | Absolute time of the sample. Lets OVRLEY sync with the video's timestamps. | , |
| `Elapsed Time (s)` | seconds | Seconds since the start of the log. | seconds |
| `Latitude`, `Longitude` | decimal degrees | Position. Needed for the route, map and distance widgets only. | degrees |
| `Altitude (m)` | m, ft | Height above sea level. | metres |
| `Speed (km/h)` | km/h, mph, m/s | Vehicle speed. | km/h |
| `RPM (rpm)` | rpm | Engine speed. | rpm |
| `Gear` | number, or `N` | Gear engaged. `N` (or `0`) is neutral. | , |
| `Throttle Position (%)` | % | Throttle opening, 0 to 100. | percent |
| `Engine Load (%)` | % | Engine load, 0 to 100. | percent |
| `Lateral Acceleration (g)` | g | Sideways acceleration, used by the G-force widget. | g |
| `Longitudinal Acceleration (g)` | g | Acceleration (+) and braking (-). | g |
| `Coolant Temperature (°C)` | °C, °F | Engine coolant temperature. | °C |
| `Intake Air Temperature (°C)` | °C, °F | Air temperature at the intake. | °C |
| `Fuel Level (L)` | litres | **New.** Fuel left in the tank. | litres |
| `Battery Voltage (V)` | volts | **New.** Battery or system voltage. | volts |
| `Estimated Range (km)` | km, mi | **New.** Distance the remaining fuel should cover. | kilometres |
| `Heading (deg)` | degrees | Direction of travel, 0 = north, 90 = east. | degrees |

Other metrics the importer understands (not in the template): `Torque`, `Power`, `Brake Position`, `Lean Angle`,
`Lap`, `Distance`, `Distance to Home`, `Barometric Altitude`, and combined or per-axis G-force.

### Time

Give at least one time column. `Elapsed Time (s)` alone is enough to draw the widgets. Add `DateTime (UTC)` when you
want OVRLEY to line the data up with the video automatically from its timestamps; otherwise you set the offset by
hand in the app.

### Fuel level, battery voltage and estimated range

These three were added by this fork.

- **Fuel level** is read in **litres**. A widget shows litres, or a percentage of the tank. The percentage needs the
  tank size, which is not in the file: set **Tank Capacity** (litres) in the widget's settings (default 21). A fuel
  column written in `%` is rejected, because it could not be converted back to litres.
- **Battery voltage** is read in volts. A reading of 0 V is treated as "the sensor did not answer" and dropped.
- **Estimated range** is read in kilometres (or miles with `(mi)`) and can be shown in either.

## How columns are recognized

- The match is by name, ignoring upper and lower case. `_` and `-` count as spaces, so `Coolant_Temp_C` and
  `coolant temp c` are the same column.
- A unit goes in parentheses at the end, `Speed (km/h)`, or in a **second row** under the header that holds only units.
- Common alternative names work, for example:
  `ECT` for coolant temperature, `IAT` for intake air temperature, `Fuel_L`, `Battery_V`, `Range_Km`, `Throttle`,
  `Gear Position`, `Elevation`, `Bearing`. The full list is in the importer
  ([`parser.rs`](../src-tauri/ovrley_core/src/activity/csv/parser.rs)).
- A source prefix such as `GPS Speed`, `OBD Speed` or `Calculated Gear` is understood; when a file has the same metric
  from several sources, the more direct one is used.
- Exports from other loggers (RaceChrono, TrackAddict, AiM, RaceBox and similar) are recognized through the same
  name rules, so you usually do not need to rename their columns.

## Readings that are dropped

A value outside the range below becomes "no reading", so one bad sample does not distort a gauge.

| Metric | Kept when |
| --- | --- |
| Latitude / longitude | -90..90 / -180..180 |
| Speed, RPM, distance | 0 or more |
| Throttle, brake, engine load | 0 to 100 |
| Coolant and intake air temperature | -60 to 300 °C |
| Fuel level | 0 to 500 L |
| Battery voltage | 0.5 to 60 V |
| Estimated range | 0 to 5000 km |

Heading is wrapped into 0 to 360. Acceleration and lean angle keep their sign.

## Checklist before importing

- The header names match the template (or an alias above), and units are in parentheses.
- Rows are in time order and there are at least two.
- Empty cells for missing readings, not zeros.
- Fuel in litres, not percent.

## Notes

- Only CSV files fill these metrics. FIT and GPX files do not carry fuel, battery or range.
- The units of a gauge's fixed scale, redline and color bands are the internal ones (litres, metres), not the unit the
  widget displays.
