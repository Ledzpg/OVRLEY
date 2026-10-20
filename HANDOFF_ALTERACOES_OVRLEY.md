# Handoff — alterações feitas no OVRLEY (fork local) para o Garagem em Dia

Este arquivo registra o que foi alterado **neste clone do OVRLEY**
(<https://github.com/spirokai/OVRLEY>, base: commit `13c3f41f`, "Merge pull
request #56 from spirokai/rust-rewrite") e a mudança correspondente feita no
projeto **Garagem em Dia**. Segue o estilo do `handoff.md` do Garagem: regras
de convivência no começo e um log por sessão no final.

Ele **complementa** o [agents.md](agents.md) (regras de código e arquitetura do
OVRLEY, que continuam valendo) e **não substitui** a leitura do código. O
`agents.md` diz que parte de `docs/` é antiga; este arquivo descreve só o que
mudou em 2026-10-20.

## Antes de tocar em qualquer coisa

1. Leia este arquivo inteiro, principalmente a seção **Log de sessões**.
2. Rode `git status` e `git log --oneline -6`. As mudanças abaixo estão
   commitadas no branch **`ledzpg`** (antes `garagem-customizacoes`; criado a partir do `main`
   no commit `13c3f41f`). Remotos: `origin` = fork `Ledzpg/OVRLEY`, `upstream` = `spirokai/OVRLEY`. Os commits, em
   ordem, são: temperaturas, sombra por widget, escala fixa/redline e este
   documento. Cada commit de código compila sozinho (`cargo check --tests`).
   Se houver arquivos modificados que você não tocou, pare e pergunte ao
   usuário antes de commitar, descartar ou sobrescrever. Não faça merge,
   rebase, reset nem push por conta própria.
3. **Nunca rode `pnpm build` / `pnpm tauri build` sem o usuário pedir**
   (regra do `agents.md`). `cargo check` e `cargo test` no `ovrley_core` são
   seguros; `pnpm dev` abre o app.
4. Respeite as regras do `agents.md`: config do usuário é contrato estrito
   (erro explícito, sem corrigir dado malformado), um único nome canônico por
   campo, validação uma vez na entrada (Rust `normalize/`), componentes React
   apresentacionais com lógica em hooks/utils.

## Ambiente preparado nesta máquina (Windows)

Foi instalado, com autorização do usuário:

- **Rust 1.95.0** via `rustup` (winget `Rustlang.Rustup`). O `cargo` fica em
  `%USERPROFILE%\.cargo\bin`; em shells novos pode ser preciso
  `$env:Path += ";$env:USERPROFILE\.cargo\bin"` (PowerShell). No Git Bash
  `cargo` pode não estar no PATH: use o PowerShell.
- **Visual Studio Build Tools 2022** com a carga de trabalho C++ (winget
  `Microsoft.VisualStudio.2022.BuildTools`), exigido pelo linker MSVC e pelo Skia.
- **pnpm 10.25.0 via corepack**: o `corepack enable` global falha com EPERM em
  `C:\Program Files\nodejs`, então os atalhos foram criados em
  `%USERPROFILE%\.corepack-bin` (`corepack enable --install-directory`).
  Em um terminal novo: `$env:Path = "$env:USERPROFILE\.corepack-bin;$env:USERPROFILE\.cargo\bin;" + $env:Path`.
  Isso é necessário porque `pnpm dev` chama `pnpm tauri dev` por dentro.
- Dependências JS instaladas com `pnpm install --frozen-lockfile`. O
  `vendor/ffmpeg/` (FFmpeg 191 MiB, BtbN FFmpeg-Builds) foi baixado pelo
  postinstall do projeto; o Tauri **exige** essa pasta para compilar.
- **`igc-parser` (dependência do GitHub) foi compilado à mão.** O pacote vem de
  um tarball do GitHub sem `index.js`; o pnpm tenta gerá-lo rodando o `pnpm
  install` do próprio pacote, que pede pnpm 12.14.0, e o corepack deste PC não
  consegue executá-lo (`ERR_PNPM_PREPARE_PACKAGE`). Sem `index.js` o Vite não
  resolve o pacote e o app não carrega. Solução usada: baixar o tarball do
  commit do lockfile, transpilar `index.ts` com o esbuild do próprio projeto e
  gravar `index.js` em `node_modules/.pnpm/igc-parser@.../node_modules/igc-parser/`.
  **Qualquer `pnpm install --force` ou reinstalação do `node_modules` apaga
  esse arquivo** e o app volta a não abrir; refaça o passo (script usado:
  `esbuild.buildSync({ entryPoints: ['index.ts'], outfile: '.../index.js',
  format: 'cjs', platform: 'node', target: 'es2017' })`).

Comandos de verificação:

```powershell
# Rust (a partir da raiz do OVRLEY)
cargo test --manifest-path src-tauri/ovrley_core/Cargo.toml --no-fail-fast

# Frontend (a partir de app/)
corepack pnpm lint
corepack pnpm exec vitest run
```

## Escopo: o que foi pedido

O usuário pediu cinco coisas, todas implementadas:

1. Importar e exibir **temperatura do arrefecimento** e **temperatura do ar de
   admissão** gravadas nos logs do cockpit do Garagem.
2. Os dois widgets novos mostram **°C** e o **ícone de termômetro** como o
   widget "Temperatura" do grupo geral.
3. **Intensidade e distância da sombra por widget** (antes só existiam nas
   configurações globais).
4. **Escala fixa e redline** nos medidores de **RPM e temperaturas**. As demais
   métricas ficaram de fora de propósito (ver "Como ligar para outras métricas").
5. Atualizar a **exportação do Garagem** para gravar as duas temperaturas.
6. Novos layouts (templates) de cluster, principalmente tacômetro + velocímetro + marcha no canto
   inferior direito, o que exigiu **marcas e numerais no mostrador** (seção 5).
7. Barras na vertical (já existia, seção 7) e avaliação de um mapa com ruas (seção 7, não implementado).

## 1. Temperaturas: `coolant_temperature` e `intake_air_temperature`

Duas métricas novas, ligadas em todo o caminho da `core_temperature`
(que serviu de modelo). Unidade canônica: **°C**.

- **Importador de CSV** (`src-tauri/ovrley_core/src/activity/csv/`):
  - `mod.rs`: `Metric::CoolantTemperature` e `Metric::IntakeAirTemperature`.
  - `parser.rs`: apelidos exatos (sem busca aproximada). Arrefecimento:
    `coolant temperature`, `coolant temp`, `coolant temp c`,
    `engine coolant temperature`, `engine coolant temp`, `ect`. Admissão:
    `intake air temperature`, `intake air temp`, `intake air temp c`,
    `intake temperature`, `intake temp`, `iat`. Os cabeçalhos são normalizados
    antes (minúsculas, `_` e `-` viram espaço), então `Coolant_Temp_C` e
    `Coolant Temperature (°C)` funcionam.
  - `units.rs`: `Unit::Celsius` / `Unit::Fahrenheit` (`c`, `°c`, `℃`, `celsius`,
    `f`, `°f`, `fahrenheit`...). Fahrenheit é convertido para °C na entrada.
  - `metrics.rs`: leituras fora de **-60..300 °C** são descartadas (sensor
    desconectado). `columns.rs` preenche as séries.
- **Modelo de dados** (mesmos pontos da `core_temperature`): `types.rs`
  (`MetricKind`), `standard_metrics.rs`, `activity/schema.rs` (4 structs),
  `finalize.rs`, `finalize/metrics.rs`, `trim.rs`, `interpolate.rs`,
  `metric_ranges.rs`, `normalize/mod.rs` (requisitos), `render/format.rs`
  (conversão °C→°F compartilhada), `vbo/mod.rs` e `mp4_telemetry/activity.rs`
  (campos vazios). **Não** foi adicionado enum de ícone novo: os widgets reusam
  `widget-temperature.svg`.
- **Catálogo** `assets/standard-metrics.json`: duas definições novas, categoria
  `motosports`, formatador `temperature`, unidades `celsius`/`fahrenheit`,
  `showUnitsByDefault: true`, ícone `{ source: shared, assetFile:
  widget-temperature.svg }` (igual ao `temperature`).
- **Frontend**: `lib/widget/standard-metrics.js` (conversão °F nos dois tipos).
  Textos só em **en** e **pt-BR** (`widgets.types.*`); os outros 13 idiomas caem
  no inglês pelo `fallbackLng`.
- Os parsers JS de FIT/GPX (`app/src/lib/activity/`) **não** preenchem essas
  métricas; só o CSV.

Testes: `tests/csv_activity.rs` (5 novos: cabeçalho OVRLEY, cabeçalhos do log
do Garagem, Fahrenheit, valores absurdos, exportação completa de 15 colunas),
`metric_kind_serde_tests.rs`, `standardMetricCatalog.test.js`,
`metricWidgetPreviewModel.test.js`.

## 2. Sombra por widget

Contrato: `shadow_strength` e `shadow_distance` **opcionais** em cada widget
de **rótulo, valor e plot**. Ausentes = herdam o valor global. Backdrops e
rasters não têm sombra e não ganharam o controle.

- **Rust**: `normalize/scene.rs` tem `ShadowOverride { strength, distance }`,
  `validate_shadow_override` (valores presentes devem ser finitos e >= 0) e os
  métodos `ValidatedScenePresentation::shadow_strength_for` /
  `shadow_distance_for`. Cada tipo validado ganhou o campo `shadow`
  (`ValidatedLabel`, `ValidatedValueWidget`, `ValidatedGradientWidget`,
  `ValidatedLinearGaugeWidget`, `ValidatedGForceWidget`,
  `ValidatedLeanAngleWidget`, `ValidatedLapTimer`, `ValidatedHeading`,
  `ValidatedRoutePlot`, `ValidatedElevationPlot`, `NormalizedElevationPlot`).
  O medidor de arco usa `inner_value.shadow` (o valor interno é o mesmo
  `ValueConfig`). Todos os pontos que liam `scene.presentation.shadow_*` agora
  chamam `*_for(&widget.shadow)`: `render/text.rs` (4 construtores),
  `gauges/arc|linear`, `g_force`, `lean_angle`, `heading`, `route`, `elevation`.
  A cor da sombra continua global.
- **Frontend**: `lib/template/template-constants.js` (`WIDGET_SHADOW_KEYS`
  entra em `LABEL_KEYS`, `VALUE_SHARED_KEYS` e nas chaves de plot, **sem**
  default no manifesto de propósito); `lib/template/template-state.js`
  materializa o valor global quando o widget não tem o seu
  (`applyGlobalShadowDefaults`); `widget-preview/shared/shadow.js`
  (`resolveWidgetSceneStyle`) e `WidgetPreview.jsx` entregam ao preview o estilo
  já resolvido; `widgetEditorSections.jsx` (`PositionSection`) ganhou os dois
  sliders (0 a 20). Textos em en e pt-BR (`widget-editor.shadowStrength` e
  `shadowDistance`).
- Os valores ficam no nível do widget (não em `display_variants`).

Testes: `widgetShadow.test.js` (frontend). No Rust a cobertura é indireta
(compilação de todos os consumidores e os testes existentes de render).

## 3. Escala fixa e redline nos medidores (RPM e temperaturas)

Campos novos no **variante** de exibição de `arc`, `corner` e `linear`, todos em
**unidade canônica** (rpm, °C), todos opcionais:

| Campo | Significado |
|---|---|
| `scale_min`, `scale_max` | escala fixa; precisam vir juntos, `max > min` |
| `redline_value` | onde começa o redline; dentro de `(min, max)` quando a escala é fixa |
| `redline_color` | cor hex, **obrigatória** quando `redline_value` existe |

Sem nenhum deles o medidor se comporta como antes (escala automática, sem
redline). O redline também é aceito com escala automática.

- **Rust**: `normalize/gauge_scale.rs` (`ValidatedGaugeScale`,
  `validate_gauge_scale`), campos `scale` em `ValidatedArcGaugeWidget` e
  `ValidatedLinearGaugeWidget`, campos novos em `ValueConfig`
  (`normalize/raw/mod.rs`). `render/widgets/gauges/metric.rs`:
  `gauge_range`, `resolve_redline`, `bar_is_redline`. `ResolvedRedline` e o
  campo `redline` nos caches (`render/widgets/types.rs`).
- **Desenho** (o redline aparece também com o medidor **vazio**): a trilha
  vazia é pintada com duas cores divididas no ponto do redline; o
  preenchimento vira duas pinturas recortadas quando passa do redline.
  - Arco/canto (`gauges/arc/path.rs`, `layer.rs`): `ArcTrackSpec::redline_zones`
    gera dois recortes em cunha usando o **mesmo cálculo com cantos
    arredondados** do `reveal_clip`, então o fim do preenchimento coincide com o
    início do redline.
  - Linear (`gauges/linear/geometry.rs::redline_zone_rects`, `layer.rs`):
    recortes retangulares no ponto onde um preenchimento de `fraction` terminaria.
  - Barras (segmentadas): a barra `i` é redline se `i / count >= fraction`
    (regra única para a trilha vazia e para o preenchimento).
- **Preview SVG** (espelha o Rust): `widget-preview/shared/gaugeRedline.js` e as
  mudanças em `arc-gauge/*` e `linear-gauge/*`. Há diferença sub-pixel entre o
  preview (geometria externa) e o Rust (geometria interna da borda); o preview
  não é pixel-perfect e não precisa ser.
- **Editor**: `metricWidget/GaugeScaleControls.jsx` (chave "Escala fixa" com
  mínimo/máximo e chave "Redline" com valor/cor) e `gaugeScaleUpdates.js`
  (puro: rejeita faixa invertida/não numérica, mantém o redline dentro da
  escala). Ligado em `ArcDisplaySection.jsx` e `LinearDisplaySection.jsx`.
  `GAUGE_SCALE_KEYS` em `template-constants.js` entra nas chaves de variante de
  `arc`, `corner` e `linear`.
- **Quais métricas mostram os controles**: as que têm `gaugeScale` em
  `assets/standard-metrics.json` com valores sugeridos `{ min, max, redline }`:
  `rpm` (0 / 12000 / 10000), `temperature`, `core_temperature`,
  `coolant_temperature` (40 / 120 / 105) e `intake_air_temperature` (0 / 80 / 60).
  Leitor: `getGaugeScaleDefaults()` em `lib/widget/standard-metrics.js`.
  O Rust aceita os campos em **qualquer** métrica de medidor; só a interface é limitada.

### Como ligar para outras métricas

Basta adicionar `"gaugeScale": { "min": ..., "max": ..., "redline": ... }` à
definição da métrica em `assets/standard-metrics.json` (valores na unidade
canônica). Nada de Rust a mudar. Candidatas que o usuário pode querer depois:
frequência cardíaca e velocidade. **Atenção**: a velocidade é canônica em
**m/s**, então os números da escala seriam em m/s, não km/h.

Testes: `tests/gauge_scale_tests.rs` (9: validação, cache, e **pixels reais**
para linear contínuo, barras e arco, com trilha cheia e vazia) e
`gaugeScaleRedline.test.jsx` (19: layout, geometria dos recortes, SVG, editor,
normalização do template).

## 4. Garagem em Dia (projeto irmão)

Pasta: `G:\Users\Luiz\Desktop\DEV PROGRAMAS\Garagem`. Commitado no branch
`unificacao` (o branch em uso lá) em dois commits:
`feat(datalogger): colunas proprias de temperatura no CSV do OVRLEY` e
`docs(handoff): ...`. Sem push e sem bump de versão.

- `app/src/cockpit/services/datalogger/ovrleyExport.ts`: o CSV agora tem **15
  colunas**. `Torque (Nm)` (que carregava a temperatura do arrefecimento como
  atalho) foi substituída por `Coolant Temperature (°C)`, e entrou
  `Intake Air Temperature (°C)`, antes de `Heading (deg)`. Falhas pontuais
  repetem a leitura anterior; sensor que **nunca** respondeu deixa a coluna
  **vazia** (não `0`), para o OVRLEY tratar como "sem dado".
- `ovrleyExport.test.ts` atualizado (15 colunas, temperaturas, coluna vazia);
  `DataloggerView.tsx`: só o comentário. `ovrley_csv_spec.md` reescrito nas
  partes de colunas, seção 4, exemplo, checklist e tabela de widgets.
- Verificado: `npx vitest run` no `app/` do Garagem (84 testes) e
  `npx tsc --noEmit` sem erros. **Não** houve build do APK/AAB nem bump de versão.
  O `handoff.md` do Garagem ganhou uma entrada apontando para este arquivo.
- Gravações exportadas **antes** desta mudança têm a temperatura na coluna
  Torque; é preciso reexportar.

## 5. Marcas e numerais do mostrador (arco e canto)

Mostrador de tacômetro: marcas maiores (com numerais), marcas menores entre elas e tudo depois do redline na
cor do redline. Só em medidores `arc` e `corner` e **exige escala fixa** (`scale_min`/`scale_max`). Todos os
valores em unidade canônica; os numerais mostram `valor / tick_label_divisor` (rpm ÷ 1000 → 0..11).

| Campo | Significado |
|---|---|
| `tick_major_interval` | liga as marcas; distância entre marcas maiores |
| `tick_minor_count` | marcas menores entre duas maiores (0 a 9) |
| `tick_position` | `inside` (para o centro) ou `outside` |
| `tick_color`, `tick_thickness`, `tick_offset` | cor, espessura e distância da trilha |
| `tick_major_length`, `tick_minor_length` | comprimentos |
| `tick_show_labels`, `tick_label_divisor`, `tick_label_decimals` | numerais |
| `tick_label_font_size`, `tick_label_color`, `tick_label_offset` | estilo dos numerais |

Com `tick_major_interval` presente **todos** os outros campos são obrigatórios (o frontend materializa os
padrões); o limite é 400 marcas. A fonte dos numerais é `min_max_label_font`.

- **Rust**: `normalize/gauge_ticks.rs` (validação, `ValidatedGaugeTicks::marks`), campo `ticks` em
  `ValidatedArcGaugeWidget`, campos novos em `ValueConfig`; `render/widgets/gauges/arc/ticks.rs` desenha tudo
  na camada estática (`draw_static_layer`). O ângulo de cada marca usa `ArcTrackSpec::angle_at`, o **mesmo**
  mapeamento do preenchimento e do redline (cantos arredondados incluídos); em barras segmentadas usa o
  varrimento linear. `redline_zones` foi refatorado para usar `angle_at`.
- **Frontend**: `widget-preview/shared/gaugeTicks.js` (+ `getArcFractionAngle` em `gaugeRedline.js`), hook e
  SVG do arco; editor em `metricWidget/GaugeTickControls.jsx` e `gaugeTickUpdates.js` (puro). Desligar a escala
  fixa também desliga as marcas. `GAUGE_TICK_KEYS` em `template-constants.js` (só `arc` e `corner`).
  `gaugeScale` em `assets/standard-metrics.json` ganhou `tickInterval`, `tickMinor` e `labelDivisor` por métrica.
- **Cuidado com i18n**: as chaves novas têm prefixo `gaugeTick*` porque `ticks` e `tickColor` já existiam
  (fita de bússola). Chave duplicada em JSON sobrescreve a anterior em silêncio.
- Testes: `tests/gauge_ticks_tests.rs` (5, incluindo pixels de marcas e o redline em **barra vertical**) e
  `gaugeTicks.test.jsx` (16).

## 6. Templates de cluster

Em `templates/` (1920x1080, fonte Rajdhani): `cluster-tacometro-velocimetro` (mostrador de RPM 0-11000 com
redline em 10000 com faixas de cor — ver seção 8 —, velocidade grande, caixa amarela da marcha, RPM pequeno), `cluster-barras-verticais`
(velocidade + marcha e duas barras verticais, RPM e arrefecimento, com redline) e `painel-temperaturas`
(dois mostradores, arrefecimento 40-120 e admissão 0-80). Os valores de escala e redline são do exemplo do
usuário (0-11000 rpm); ajuste na interface para outras motos. O número da marcha usa a **sombra por widget**
com valor 0. `app/src/tests/features/template-manager/bundledClusterTemplates.test.js` garante que carregam
pelo mesmo caminho do app e que o backend recebe uma config completa.

**Como conferir um template visualmente sem abrir o app** (usado nesta sessão; os arquivos foram descartados):
um teste vitest temporário carrega o JSON com `normalizeTemplateFilePayload` e grava
`createBatchRenderTemplate(...)` em disco; um teste Rust `#[ignore]` lê essa config, importa um CSV real com
`parse_csv_activity_path`, faz `trim_activity` + `densify_activity` e chama `render_preview_with_report`
para gerar um PNG (composto sobre um fundo cinza para as cores claras aparecerem).

## 7. Barras na vertical e mapa

- **Barras verticais já existiam**: o medidor linear tem o campo `orientation` (`horizontal`/`vertical`) no
  editor ("Orientação"); o preenchimento sobe de baixo para cima. Agora o redline também foi testado na
  vertical (`vertical_linear_gauge_puts_the_redline_zone_at_the_top`) e há um template que a usa.
- **Mapa com ruas**: implementado como fundo de mapa raster do widget de rota — ver seção 9.

## 8. Faixas de cor, degradê e redline cheio/piscante

Para o tacômetro (e qualquer medidor de arco, canto ou linear com **escala fixa**): o preenchimento pode ter
faixas de cor, sólidas ou em degradê, e ao chegar no redline o medidor inteiro pode ficar vermelho e piscar.

| Campo | Significado |
|---|---|
| `fill_zones` | lista de 1 a 12 faixas `{ start, color, end_color? }`; a primeira começa em `scale_min`, `start` estritamente crescente e menor que `scale_max` |
| `redline_fill_all` | ao atingir `redline_value` pinta a trilha toda na cor do redline (exige `redline_value`) |
| `redline_blink_hz` | pisca nessa frequência (0 < hz ≤ 20; exige `redline_fill_all`) |

- Faixa sem `end_color` é sólida; com `end_color` o degradê vai da cor da faixa até o início da próxima. **A última
  faixa termina no redline** (quando existe e cai dentro dela), então um degradê amarelo → vermelho a partir de 8000
  encontra o vermelho do redline exatamente em 10000; sem redline, vai até o fim da escala. `end_color` da última
  faixa deve ser a cor do redline para a emenda ficar contínua (o editor já sugere isso).
- Exemplo do usuário (moto: torque máximo 7000, potência máxima 8000, redline 10000): `[0 #2bd9ff→#47ff6b]`,
  `[7000 #47ff6b]`, `[8000 #ffd400→#ff3b30]`. O editor tem o botão de preset por torque/potência
  (`buildTorquePowerZones`) e o `standard-metrics.json` traz `zonePreset` para o rpm.
- **Rust**: `normalize/gauge_scale.rs` (`ValidatedFillZone`, `ValidatedRedline` com `fill_all`/`blink_hz`),
  `render/widgets/gauges/metric.rs` (`fill_stops`, `redline_overrun`: sobre o redline desenha a trilha toda; fase
  "apagada" do pisca esconde o preenchimento; o tempo vem do tempo do quadro). Arco usa gradiente de varredura
  (Skia), linear usa gradiente linear e barras segmentadas amostram a cor no centro de cada barra.
- **Frontend**: `widget-preview/shared/gaugeZones.js` (`getFillStops(zones, range, redlineValue)`,
  `getStopColorAt`, `isRedlineOverrun`, `isRedlineBlinkOn`), editor `GaugeZoneControls.jsx` +
  `gaugeZoneUpdates.js`. O arco do preview aproxima o gradiente de varredura com cunhas de 3°.
- Testes: `tests/gauge_zones_tests.rs` (8: validação, gradiente/sólido, redline cheio, pisca ligado/apagado,
  arco, e o degradê da última faixa chegando ao vermelho no redline) e `gaugeZones.test.jsx`.

## 9. Mapa raster (tiles) atrás da rota

O widget `course` ganhou um fundo de mapa opcional feito de **tiles raster** (PNG/JPEG, 256 px, esquema XYZ), como
o GPStitch/gopro-overlay. Sem `map_tile_url` o widget é idêntico ao de antes.

| Campo | Significado |
|---|---|
| `map_tile_url` | modelo de URL com `{z}`, `{x}` e `{y}` (qualquer ordem); só `https://` ou `file://` (pacote de tiles offline) |
| `map_attribution` | crédito desenhado no canto inferior direito (obrigatório, não vazio) |
| `map_opacity` | 0-100 (o Rust aceita fração ou percentual; abaixo de 5 o editor não deixa, porque 1 seria lido como 100%) |
| `map_zoom_offset` | -2 a 2 somados ao zoom que casa com a resolução do widget (positivo = mais nítido, mais tiles) |

Contrato estrito: se `map_tile_url` existe, os outros três são obrigatórios; os outros três sem a URL são erro.

- **Projeção**: com mapa, a rota usa **Web Mercator** (`mercator_degrees`: x = lon+180, y = ordenada de Mercator
  em graus, mundo = 360x360) em vez da aproximação equirretangular; sem mapa nada muda. A rota e o mapa usam a
  **mesma** transformação de ajuste (`render/widgets/route/fit.rs::RouteFit`), então a linha cai exatamente sobre
  as ruas. Zoom = o que faz o pixel do tile casar com o pixel de saída (+ `map_zoom_offset`), reduzido até caber
  em no máximo 64 tiles. Os tiles viram um mosaico e são desenhados de uma vez com interpolação linear.
- **Rust (`ovrley_core`)**: `raster_tiles.rs` (trait `RasterTileSource`, registro global
  `install_raster_tile_source`, modelos de URL, leitura de `file://`), `render/widgets/route/map.rs` (zoom, busca
  de tiles em até 6 threads, mosaico, camada estática girada com o widget, crédito), `fit.rs`, `prepare.rs`
  (`route_map_fit`, `rotated_layer_bounds`, projeção Mercator), `normalize/route.rs` (`ValidatedRouteMap`),
  `commands/route_geometry.rs::build_route_map_png_command` (PNG para o preview, sem o crédito). **Falha tile =
  falha o render** (mensagem com a URL), sem mapa parcial.
- **Tauri (`src-tauri`)**: `raster_tile_source.rs` (`HttpRasterTileSource`: reqwest com User-Agent
  `OVRLEY-luizleite/<versão>`, cache em disco em `<app cache>/raster-tiles`, validade de 14 dias, usa o tile vencido se não
  conseguir atualizar, recusa resposta que não seja imagem), instalado em `lib.rs` no setup; comando
  `backend_build_route_map` devolve um data URL PNG (`null` se não há mapa).
- **Frontend**: `COURSE_MAP_KEYS` em `template-constants.js`; `usePlotPreviewGeometry.js` busca a imagem
  (`buildRouteMap` em `api/backend.js`) e `RoutePreview.jsx` a desenha com o crédito; editor em
  `RouteMapBackgroundSection.jsx` + `routeMapUpdates.js` (presets, validação). O preview só consegue
  mostrar o mapa com o app Tauri (usa o comando).
- **Provedores**: os presets são só os que respondem **sem chave**: OpenStreetMap, OpenTopoMap e Esri Light Gray.
  **Os tiles raster do CARTO (`basemaps.cartocdn.com`) agora devolvem uma marca d'água "API KEY REQUIRED"** —
  foi o primeiro teste e por isso foram removidos. Provedores com chave (Thunderforest, MapTiler, Stadia, Mapbox)
  funcionam pela opção "URL personalizada" com a chave na própria URL (fica salva no template: não compartilhar).
  Cada provedor tem termos de uso e exige o crédito; o OSM pede uso moderado e User-Agent identificável (o cache
  em disco ajuda: cada tile é baixado uma vez). Confira os termos do Esri antes de usar em vídeo público.
- Template `templates/mapa-trajeto.json` (mapa OSM, linha ciano, marcador em halo amarelo). A seta amarela da
  imagem de referência **não existe**: os marcadores são `single`, `ring` e `halo`.
- Testes: `tests/route_map_tests.rs` (6: alinhamento por pixel com um pacote `file://` gerado, PNG do preview,
  opacidade, rota sem mapa inalterada, validação estrita, falha de tile alto e claro), testes unitários de
  `fit`, `map` e `raster_tiles`, `raster_tile_source.rs` (4, com servidor `tiny_http` local) e
  `routeMapUpdates.test.js` (9). Conferido **visualmente** com tiles reais do OSM sobre o `ida_ovrley_temps.csv`:
  a rota segue a RJ-124 (Rio Bonito -> Araruama).
- Rodar `cargo test` na pasta `src-tauri` (crate do app) exige
  `TAURI_CONFIG='{"bundle":{"resources":{"../THIRD_PARTY_NOTICES.txt":null}}}'` porque o arquivo de licenças só
  é gerado no empacotamento (é o mesmo atalho que `scripts/tauri.mjs` usa no `dev`).
- Limitações: sem cache de tiles entre execuções do `ovrley_core` fora do app (só o shell Tauri instala a fonte
  HTTP); mapa gira junto com a rotação do widget (o texto dos tiles também); não há estilo claro/escuro por filtro
  de cor; tiles do hemisfério com antimeridiano não são tratados.

## 10. Agulha do mostrador (arco e canto)

Barra curta com o miolo translúcido e a borda sólida, atravessando a trilha no ângulo do valor atual, como a agulha
do tacômetro da imagem de referência. Só em medidores `arc` e `corner`; funciona com escala automática ou fixa.

| Campo | Significado |
|---|---|
| `needle_length` | liga a agulha; tamanho ao longo do raio |
| `needle_width` | tamanho transversal |
| `needle_offset` | deslocamento do centro da agulha a partir do meio da trilha (positivo = para fora, aceita negativo) |
| `needle_color` | cor da borda (e do miolo quando não há `needle_fill_color`) |
| `needle_fill_color` | opcional: cor do miolo (ou da ponta de dentro, no degradê); ausente = `needle_color` |
| `needle_fill_end_color`, `needle_fill_end_opacity` | opcionais, **juntos**: ponta de fora do degradê (cor e opacidade 0-1); a opacidade sem a cor é erro |
| `needle_border_thickness` | espessura da borda (0 = sem borda) |
| `needle_fill_opacity` | opacidade do miolo, ou da ponta de dentro no degradê, 0-1 (0 deixa só a borda) |
| `needle_corner_radius` | raio dos cantos (limitado à metade da agulha) |

Com `needle_length` presente **todos** os outros campos são obrigatórios (o frontend materializa os padrões).

O degradê corre ao longo da agulha, do lado do centro até a ponta de fora, interpolando cor e opacidade de cada ponta
(`needle.rs::fill_paint`, gradiente linear do Skia; no preview, `<linearGradient>` em `userSpaceOnUse`). Ligar o degradê
no editor começa com a cor do miolo e opacidade 0 na ponta de fora (efeito de brilho que some).

- **Rust**: `normalize/gauge_needle.rs` (`ValidatedGaugeNeedle`), campo `needle` em `ValidatedArcGaugeWidget`;
  `render/widgets/gauges/arc/needle.rs` desenha a cada quadro por cima do preenchimento (e sob o valor numérico).
  O ângulo vem de `layer.rs::value_angle`/`value_track_spec`, o **mesmo** mapeamento das marcas (extraído de
  `draw_static_layer`): cap-aware no preenchimento contínuo, varredura simples nas barras. A agulha continua
  marcando o valor real quando o redline enche a trilha toda.
- **Frontend**: `shared/gaugeNeedle.js` (+ `getArcValueAngle` extraído de `gaugeTicks.js`), hook e SVG do arco,
  editor `GaugeNeedleControls.jsx` + `gaugeNeedleUpdates.js`, `GAUGE_NEEDLE_KEYS` em `template-constants.js`
  (só `arc` e `corner`). Chaves de i18n com prefixo `gaugeNeedle*`.
- O template `cluster-tacometro-velocimetro` usa a agulha (40 x 10, deslocada 6 px para fora, miolo a 30%).
- Testes: `tests/gauge_needle_tests.rs` (10: validação, posição, segue o valor, sem agulha, só borda, cor própria do
  miolo, degradê da ponta de dentro para a de fora, cor/opacidade de cada ponta) e `gaugeNeedle.test.jsx` (26). Conferido visualmente no cluster com dados reais.

## 11. Conjuntos de widgets no menu

No drawer de widgets há a seção **Conjuntos** (`WidgetButtonGrid`), que adiciona vários widgets de uma vez:
`tach-cluster` (tacômetro + velocímetro + marcha, de `templates/cluster-tacometro-velocimetro.json`) e
`temperature-panel` (arrefecimento + admissão, de `templates/painel-temperaturas.json`). O layout vem do próprio
template (fonte única): `useWidgetManager.addWidgetGroup` lê o template (`backend.getTemplate`), passa por
`prepareTemplateFilePayload` (fontes) e `features/widget-editor/utils/widgetGroupConfig.js::appendWidgetGroup`
acrescenta backdrops, labels, valores e plots à config atual **descartando os ids do template** (ids novos e únicos),
seleciona o primeiro e fecha o drawer; erro de leitura vai para `setErrorMessage` e nada muda. Catálogo em
`features/widget-drawer/data/widgetGroups.js` (para um novo conjunto: uma entrada + chaves `widget-drawer.*` no i18n).
Os widgets entram nas posições do template (1920x1080); em cenas de outro tamanho é preciso arrastar.
Os templates continuam existindo na lista de templates. O `cluster-barras-verticais` e o `mapa-trajeto` não estão
no menu (o mapa já é o widget "Map" com as opções de mapa). Testes: `WidgetGroups.test.jsx` (12).

## 11b. Fontes dos templates e limite do ajuste vertical

- **Erro "font family is unavailable: Bebas Neue"** ao carregar um template: nome de família "solto" (sem extensão) é
  procurado entre as fontes **do sistema**, e os templates `champagne-borders`, `champagne-shadows` e `white-opacity`
  usavam "Bebas Neue" (e o `lavender-gradient`, "Roboto"), que só existem na máquina de quem os fez. Correção:
  `fonts/BebasNeue-Regular.ttf` (+ `fonts/OFL-BebasNeue.txt`, licença OFL) entrou no pacote e os templates passaram a
  usar os ids embutidos `Bebas Neue.ttf` e `Inter.ttf` (o `lavender-gradient` perdeu o Roboto). Teste
  `tests/bundled_template_fonts_tests.rs`: todo template em `templates/` só pode citar fontes embutidas. Projetos ou
  templates **do usuário** que citem "Bebas Neue" solto continuam falhando: trocar a fonte para "Bebas Neue.ttf".
- Slider "Ajuste vertical" do rótulo do mostrador de arco (`ArcDisplaySection.jsx`): limite de ±50 para ±150 px
  (o cluster usa 96).

## 12. Mapa aproximado (segue a posição atual)

Widget novo **"Mapa aproximado"** (`value: "course_zoom"`, botão "Map+" no drawer): tiles raster centrados na posição
atual num zoom fixo, a rota por cima (trecho percorrido e restante em estilos próprios) e uma **seta** no centro. O
mapa pode ficar com o norte para cima (a seta aponta a direção) ou **girar** para a direção ir sempre para cima.
O mapa total (`course` com mapa, seção 9) continua como estava. Todos os campos são obrigatórios; os padrões vêm de
`assets/standard-widgets.json` (`plot.definitions.course_zoom`) e um teste Rust garante que passam na validação.

| Campo | Significado |
|---|---|
| `zoom` | 8-20, passo livre: quanto chão aparece (maior = mais perto). Em px de saída: `256 * 2^zoom * escala / 360` por grau |
| `map_zoom_offset` | -2..2, **detalhe**: baixa tiles de nível `zoom + offset` e os desenha reduzidos (mesma área, mais detalhe/rótulos menores) |
| `map_tile_url`, `map_attribution`, `map_opacity` | iguais ao mapa total (https/file, crédito obrigatório) |
| `rotate_with_heading` | gira o mapa com a direção; senão a seta gira |
| `corner_radius` | cantos arredondados do recorte |
| `completed_line_*`, `remaining_line_*` | largura, cor, opacidade das duas partes da rota |
| `marker_size/color/opacity`, `marker_outline_color/width` | a seta (`draw_marker_path`) |
| `show_full_activity` | como no mapa total (trecho exportado ou atividade inteira) |

- **Rust** (`render/widgets/route_zoom/`): `geometry.rs` (projeção para pixels de tile, `view_scale`, posição por
  progresso, direção pela tangente da rota numa janela de 40 px, suavizada no tempo, 0,35 s), `tiles.rs` (`TileStore`,
  planejamento dos tiles de todos os quadros numa grade de meio tile), `prepare.rs` (cache e estado por quadro),
  `draw.rs` (por quadro: tiles visíveis, trechos da rota perto da vista, seta, crédito). Coordenadas são desenhadas
  **relativas ao centro** em f64 (pixels de tile absolutos passam de 100 milhões, mais do que f32 resolve). Config em
  `normalize/route_zoom.rs`, `ValidatedRenderConfig.course_zoom_plots`, `PreparedRenderAssets.route_zoom_caches`.
- **Download**: no render do vídeo todos os tiles dos quadros são baixados antes (cache em disco do shell Tauri);
  **limite de 500 tiles** por mapa: acima disso o render falha com a mensagem "lower the map zoom or detail, or export
  a shorter range" (política de uso do OSM, memória). Um trajeto longo em zoom alto estoura o limite.
- **Prévia do editor**: comando `backend_build_route_zoom_frame` (config, atividade, progresso 0-1) devolve o quadro
  inteiro desenhado pelo Rust como PNG, só com os tiles daquele quadro; `useRouteZoomPreview.js` escolhe o progresso
  como o marcador do mapa total e espera 150 ms depois de cada mudança. Só funciona no app Tauri.
- **Frontend**: `RouteZoomWidgetEditor.jsx` + `routeZoomUpdates.js` (zoom, cantos, linhas, seta); o bloco de mapa
  reusa `RouteMapBackgroundSection` (`alwaysOn`) e `buildMapUpdate`. Sem controles de sombra nem rotação do widget.
  `COURSE_ZOOM_PLOT_KEYS` em `template-constants.js`, ícone `assets/widget-icons/widget-course-zoom.svg`.
- Conferido **visualmente** com tiles reais do OSM e o `ida_ovrley_temps.csv` (norte para cima e girando).
- Testes: `tests/route_zoom_tests.rs` (13: posição, zoom/detalhe, seta, mapa girando, duas partes da rota, crédito,
  validação estrita, caminho do vídeo, limite de tiles, falha de tile, padrões do editor), unitários de `geometry` e
  `tiles`, e `routeZoom.test.jsx` (11). A seta não existia como marcador do mapa total (`single/ring/halo`): continua
  só neste widget.

## 13. Atualização própria (independente do OVRLEY oficial)

- **Endereço**: `src-tauri/tauri.conf.json` → `plugins.updater.endpoints` aponta para
  `https://github.com/ledzpg/OVRLEY/releases/latest/download/latest.json` (fork do usuário; antes: releases do
  desenvolvedor original). O código React (`useAppUpdate.js`) não mudou: só pergunta ao plugin se há versão nova.
- **Assinatura**: par de chaves próprio. `pubkey` no `tauri.conf.json`; a chave **privada** fica em
  `%USERPROFILE%\.tauri\ovrley-luizleite.key` (sem senha, fora do repositório). **Fazer backup**: sem ela as
  cópias instaladas deixam de aceitar atualizações e precisam ser reinstaladas na mão.
- **Identidade**: `identifier` = `io.github.ledzpg.ovrley` e `productName` = `com.ovrley.luizleite` (antes
  `com.ovrley.desktop` / `OVRLEY`). Configurações (`%APPDATA%\io.github.ledzpg.ovrley\ovrley-settings.json`) e cache
  de tiles passam a ser separados do oficial. Os **templates e projetos do usuário não mudam**: ficam em
  `Documentos\OVRLEY\templates` e `\projects` (caminho fixo no código), então a versão nova já os enxerga.
- **User-Agent dos tiles**: `OVRLEY-luizleite/<versão> (+https://www.luizleite.eng.br/garagem/ovrley)` nos dois
  clientes (`raster_tile_source.rs` e `map_tile_service.rs`), para que servidores públicos de tiles identifiquem
  este app, e não o oficial.
- **Publicar uma versão**: `build-installer.bat` (assina com a chave acima) e, no fim, roda
  `scripts/prepare-update-site.mjs`, que cria `update-site/` com `latest.json` + instalador
  (`<productName>_<versão>_x64-setup.exe`) e mostra o comando
  `gh release create v<versão> update-site/latest.json "update-site/<instalador>" --title v<versão>`. A tag é
  `v<versão>`. Notas da versão: variável `OVRLEY_NOTES` antes de rodar o `.bat`. Só Windows x64 é publicado. O fork
  precisa ser **público** e o `gh` autenticado (`gh auth login`).
- **Versão**: a partir daqui +0.0.1 a cada conjunto de commits, com
  `node scripts/set-release-version.mjs X.Y.Z` (atualiza os 5 arquivos). Este conjunto: 0.1.1.
- **Primeira instalação**: manual (a que hoje está instalada ainda usa a chave e o endereço do oficial). Para testar
  o fluxo: instalar a 0.1.1, publicar a 0.1.2 e abrir o app.
- Licença do projeto: GPL-3.0-or-later (`LICENSE.md`); o fork público cumpre a obrigação de oferecer o código.
- Não testado de ponta a ponta (nenhum build/instalador foi gerado nem publicado).

## Notas / achados fora de escopo

- **Testes de frontend**: com o `igc-parser` compilado (ver "Ambiente"), a
  suíte inteira passa **exceto** `features/toolbar/ActivityDrawerContent.test.jsx`
  (1 teste), que espera "42.2 km" e falha porque o Windows está em pt-BR
  (formata "42,2"). Não tem relação com estas mudanças. Antes do `igc-parser`
  compilado, 7 outras suítes falhavam pelo mesmo motivo do pacote ausente.
- O log **cru** do Garagem (`Time_Sec`, `Timestamp_UTC`...) ainda não é
  importável direto: o cabeçalho de tempo `time sec` não é um apelido. Os
  cabeçalhos de temperatura do log cru já funcionam. Para importar o log cru
  seria preciso aceitar `Time_Sec` e `Timestamp_UTC`.
- O medidor de **canto** compartilha o código do arco, mas não tem teste de
  pixel próprio (só o arco e o linear têm).
- Idiomas além de en e pt-BR não ganharam as traduções novas (caem no inglês).
- O atributo `Heading` do OVRLEY é suavizado na importação (EMA circular de 1 s),
  então o valor importado difere ligeiramente do CSV.
- Nada foi testado na **interface real** do app (`pnpm dev`) nem com render de
  vídeo; a verificação foi por testes automatizados e leitura do código.

---

## Log de sessões

### 2026-10-20 (continuação 7) — Claude Code (mapa aproximado)

- Widget "Mapa aproximado" que segue a posição atual, com zoom, detalhe e seta. Seção 12.

### 2026-10-20 (continuação 6) — Claude Code (fontes, ajuste vertical, degradê da agulha)

- Fonte Bebas Neue embutida e templates corrigidos; limite do ajuste vertical 150 px; degradê na agulha. Seções 10 e 11b.

### 2026-10-20 (continuação 5) — Claude Code (conjuntos no menu)

- Seção **Conjuntos** no drawer de widgets. Seção 11.

### 2026-10-20 (continuação 4) — Claude Code (agulha)

- **Agulha do mostrador** (Rust, preview, editor, template do cluster). Seção 10. `cargo test`, `eslint` e `vitest` sem novas falhas.

### 2026-10-20 (continuação 3) — Claude Code (mapa raster)

- **Mapa de tiles raster atrás da rota** (Rust, Tauri, preview, editor, template `mapa-trajeto`). Seção 9.
- Achado: tiles raster do CARTO exigem chave de API agora; presets só com provedores sem chave.
- Verificação: `cargo test` do `ovrley_core` (57 suítes) e do app (47 testes) sem falhas; `eslint` limpo;
  `vitest` só com a falha de idioma já descrita. Conferido visualmente com tiles reais do OSM. Não testado
  na interface real (`pnpm dev`) nem com render de vídeo completo.

### 2026-10-20 (continuação 2) — Claude Code (faixas de cor)

- **Faixas de cor com degradê, redline cheio e piscante** (Rust, preview, editor, template do cluster). Seção 8.
- A última faixa termina no redline, para o degradê encontrar a cor do redline.
- Verificação: `cargo test` 56 suítes ok, `eslint` limpo, `vitest` só com a falha de idioma já descrita.

### 2026-10-20 (continuação) — Claude Code (marcas do mostrador, templates de cluster)

- **Marcas e numerais** no arco/canto (Rust, preview SVG, editor, manifesto). Seção 5.
- **3 templates novos** conferidos visualmente com dados reais do `ida_ovrley_temps.csv`. Seção 6.
- **Barras verticais**: já existiam; adicionado teste de pixel do redline na vertical. Seção 7.
- **Mapa**: estudado (app, GPStitch, gopro-overlay), não implementado. Seção 7.
- Verificação: `cargo test` do `ovrley_core` sem falhas; `eslint` limpo; `vitest` só com a falha de formatação
  de idioma já descrita. Não testado na interface real nem com render de vídeo.

### 2026-10-20 — Claude Code (temperaturas, sombra por widget, tacômetro, exportação do Garagem)

- Preparado o ambiente (Rust, Build Tools, pnpm via corepack) e clonado o OVRLEY
  na pasta `OVRLEY`. Detalhes em "Ambiente preparado nesta máquina".
- **Temperaturas** do arrefecimento e da admissão: importador de CSV, modelo de
  dados, catálogo, textos. Seção 1.
- **Sombra por widget**: contrato opcional com herança do global, no Rust, no
  preview e no editor. Seção 2.
- **Escala fixa e redline** em medidores de arco, canto e linear, liberados para
  RPM e temperaturas. Seção 3.
- **Garagem**: exportação com colunas próprias de temperatura, testes e
  especificação atualizados. Seção 4.
- Verificação: `cargo test` do `ovrley_core` sem falhas (suíte inteira);
  `eslint` limpo; `vitest` com 1 falha de formatação de idioma (ver acima); `vitest` e
  `tsc` do Garagem sem erros.
- Commits feitos no fim da sessão, a pedido do usuário: 4 no OVRLEY (branch
  `garagem-customizacoes`, hoje `ledzpg`) e 2 no Garagem (branch `unificacao`), sem push.
- **Não feito**: push, build/empacotamento, teste visual no app, teste de
  render de vídeo, tradução dos demais idiomas.
- Arquivos novos: `src-tauri/ovrley_core/src/normalize/gauge_scale.rs`,
  `src-tauri/ovrley_core/tests/gauge_scale_tests.rs`,
  `app/src/features/widget-preview/shared/gaugeRedline.js`,
  `app/src/features/widget-editor/components/metricWidget/GaugeScaleControls.jsx`,
  `.../gaugeScaleUpdates.js`, `app/src/tests/features/widget-preview/widgetShadow.test.js`,
  `app/src/tests/features/widget-preview/gaugeScaleRedline.test.jsx` e este arquivo.
