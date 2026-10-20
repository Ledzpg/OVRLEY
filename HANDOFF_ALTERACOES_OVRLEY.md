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
   commitadas no branch **`garagem-customizacoes`** (criado a partir do `main`
   no commit `13c3f41f`), sem push; não há remoto próprio. Os commits, em
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
  `garagem-customizacoes`) e 2 no Garagem (branch `unificacao`), sem push.
- **Não feito**: push, build/empacotamento, teste visual no app, teste de
  render de vídeo, tradução dos demais idiomas.
- Arquivos novos: `src-tauri/ovrley_core/src/normalize/gauge_scale.rs`,
  `src-tauri/ovrley_core/tests/gauge_scale_tests.rs`,
  `app/src/features/widget-preview/shared/gaugeRedline.js`,
  `app/src/features/widget-editor/components/metricWidget/GaugeScaleControls.jsx`,
  `.../gaugeScaleUpdates.js`, `app/src/tests/features/widget-preview/widgetShadow.test.js`,
  `app/src/tests/features/widget-preview/gaugeScaleRedline.test.jsx` e este arquivo.
