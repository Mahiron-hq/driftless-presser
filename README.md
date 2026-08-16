<div align="center">

<img src="assets/icon.ico" width="88" alt="Driftless Presser icon">

# Driftless Presser

**Лёгкий автокликер для Windows с точным таймингом, глобальными хоткеями и треем**
<br>
**Lightweight Windows auto-presser with drift-free timing, global hotkeys and tray**

[![Release](https://img.shields.io/github/v/release/Mahiron-hq/driftless-presser?style=flat-square&color=6aa84f)](https://github.com/Mahiron-hq/driftless-presser/releases/latest)
[![License](https://img.shields.io/badge/license-Apache%202.0-green?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D6?style=flat-square&logo=windows&logoColor=white)](#)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)

### [⬇️ Скачать / Download](https://github.com/Mahiron-hq/driftless-presser/releases/latest)

**[Русский](#-русский)** · **[English](#-english)**

</div>

---

## 🇷🇺 Русский

Driftless Presser автоматически нажимает выбранную клавишу с заданной частотой, пока включён. Одно окно настроек, иконка в трее, один `.exe` - без установщика, без фоновых служб, без телеметрии.

Главное отличие от типичных автокликеров - **планировщик без дрейфа**. Наивный `sleep(интервал + случайная задержка)` делает реальную частоту ниже заданной и накапливает рассинхрон со временем. Здесь удерживается фиксированная временная сетка: если интервал затянулся, следующий укорачивается, и средняя частота остаётся ровно той, что вы указали.

### Возможности

| | |
|---|---|
| ⏱️ **Точный тайминг** | Средняя частота совпадает с заданной с погрешностью < 0.2 мс на дистанции тысяч нажатий |
| 🎲 **Естественный джиттер** | 10–20 мс случайного разброса на каждое нажатие — ритм не выглядит машинным |
| 🖱️ **Вторая частота под ПКМ** | Пока зажата правая кнопка мыши, используется отдельная частота |
| ⌨️ **Глобальный хоткей** | Комбинация из 2-3 клавиш вкл/выкл, работает поверх любого активного окна |
| ⏺️ **Запись клавиш** | Нажмите на поле и просто нажмите нужные клавиши — без ручного ввода имён |
| 🛡️ **Защита от опасных комбинаций** | Ctrl+Alt+Delete, Alt+F4, Win+R, Win+L и другие системные сочетания отклоняются |
| 🌗 **Тёмная тема** | Заголовок окна подхватывает системную тему Windows |
| 🌐 **Русский и английский** | Язык определяется по локали системы автоматически |
| 🔌 **0% CPU в простое** | Поток блокируется на канале, а не крутит цикл ожидания |
| 📦 **Один файл** | ~1 МБ, LTO + strip, никаких зависимостей и рантаймов |

### Установка

1. Скачайте `.exe` из [последнего релиза](https://github.com/Mahiron-hq/driftless-presser/releases/latest).
2. Положите его в отдельную папку — рядом будет создан `config.json`.
3. Запустите. Установка не требуется.

> **SmartScreen.** Сборка не подписана сертификатом, поэтому Windows может показать предупреждение. «Подробнее» → «Выполнить в любом случае». Исходники полностью открыты - при желании соберите сами.

### Как пользоваться

1. **Целевая клавиша** - какая клавиша будет нажиматься.
2. **Частота по умолчанию (Гц)** - нажатий в секунду в обычном состоянии.
3. **Частота при ПКМ (Гц)** - нажатий в секунду, пока зажата правая кнопка мыши.
4. **Горячая клавиша вкл/выкл** - 2–3 клавиши, минимум один модификатор.
5. **Старт** - сохраняет настройки и сворачивает приложение в трей.

Дальше приложение живёт в трее. Хоткей включает и выключает нажатия из любого места. Правый клик по иконке в трее - «Открыть настройки» и «Выход».

### Конфигурация

Файл `config.json` создаётся рядом с `.exe` при первом запуске. Его можно править вручную; при повреждении файла применяются значения по умолчанию.

```json
{
  "target_key": "E",
  "default_frequency_hz": 4.0,
  "rmb_frequency_hz": 7.0,
  "hotkey": ["Ctrl", "W", "S"]
}
```

| Поле | Тип | Описание |
|---|---|---|
| `target_key` | строка | Нажимаемая клавиша: `A`-`Z`, `0`-`9`, `F1`-`F24`, `Space`, `Enter`, `Tab`, `Mouse4`, `Mouse5` и др. |
| `default_frequency_hz` | число | Частота нажатий в секунду в обычном режиме |
| `rmb_frequency_hz` | число | Частота нажатий, пока зажата ПКМ |
| `hotkey` | массив строк | 2-3 клавиши, минимум один модификатор (`Ctrl`, `Alt`, `Shift`, `Win`) |

**Правила хоткея:** от 2 до 3 клавиш, без повторов, обязателен модификатор — иначе комбинация срабатывала бы при обычном наборе текста. Системные сочетания (`Ctrl+Alt+Delete`, `Alt+Tab`, `Win+R`, `Ctrl+Shift+Esc` и другие) заблокированы, включая любые их расширения.

### Сборка из исходников

```bash
git clone https://github.com/Mahiron-hq/driftless-presser.git
cd driftless-presser
cargo build --release
```

Готовый бинарник - в `target/release/`. Профиль релиза: `opt-level = 3`, LTO, `codegen-units = 1`, `panic = "abort"`, `strip`.

Тесты платформонезависимого ядра (конфиг, парсинг клавиш, математика планировщика) запускаются на любой ОС:

```bash
cargo test --lib
```

### Как это устроено

Три потока с чёткими границами ответственности:

- **input-hook** - низкоуровневые хуки `WH_KEYBOARD_LL` и `WH_MOUSE_LL` в собственном цикле сообщений. Колбэк делает минимум работы: Windows отключает хук, который отвечает слишком долго, задерживая ввод во всей системе. Собственные нажатия помечаются через `dwExtraInfo` и игнорируются хуком.
- **clicker** - планировщик и `SendInput`. В простое блокируется на `recv()`, в активном режиме использует `recv_timeout`, чтобы выключение или смена состояния ПКМ срабатывали мгновенно, а не после текущего интервала.
- **main/GUI** - окно настроек и трей на `native-windows-gui`.

Нажатия отправляются скан-кодами, а не виртуальными кодами: так их видят движки вроде SDL2, которые читают ввод на уровне железа. Между down и up выдерживается 30 мс - некоторые игры опрашивают состояние клавиатуры раз в кадр и мгновенную пару просто не заметят.

### Ограничения

- Только Windows 10/11 - приложение опирается на Win32 API, у которых нет аналогов на других платформах (сборка под не-Windows останавливается на `compile_error!`).
- Если целевое приложение запущено от имени администратора, запустите от администратора и Driftless Presser - иначе Windows не пропустит синтетический ввод в его окно.
- Античиты воспринимают автоматизацию ввода как нарушение. Используйте инструмент в одиночных играх, в софте и там, где это разрешено правилами; ответственность за использование лежит на вас.

### Лицензия

Apache License 2.0 - см. [LICENSE](LICENSE).

---

## 🇬🇧 English

Driftless Presser repeatedly presses a key of your choice at a configured rate while it's on. One settings window, a tray icon, a single `.exe` — no installer, no background service, no telemetry.

What sets it apart from typical auto-clickers is the **drift-free scheduler**. A naive `sleep(interval + jitter)` loop runs slower than the configured rate and drifts without bound. This one keeps a fixed-step timeline instead: an interval that runs long is repaid by a shorter one, so the average rate stays exactly where you set it.

### Features

| | |
|---|---|
| ⏱️ **Drift-free timing** | Average interval matches the configured rate within < 0.2 ms across thousands of presses |
| 🎲 **Natural jitter** | 10–20 ms of randomness per press, so the cadence isn't perfectly mechanical |
| 🖱️ **Second rate on RMB** | A separate frequency applies while the right mouse button is held |
| ⌨️ **Global hotkey** | 2–3 key toggle combo that works over any focused window |
| ⏺️ **Press-to-record** | Click a field and press the keys — no typing key names by hand |
| 🛡️ **Reserved-combo guard** | Ctrl+Alt+Delete, Alt+F4, Win+R, Win+L and other system shortcuts are rejected |
| 🌗 **Dark mode** | Title bar follows the Windows system theme |
| 🌐 **English & Russian** | Language picked automatically from the system locale |
| 🔌 **0% CPU when idle** | The worker blocks on a channel instead of spinning |
| 📦 **Single file** | ~1 MB, LTO + strip, no runtime or dependencies to install |

### Install

1. Grab the `.exe` from the [latest release](https://github.com/Mahiron-hq/driftless-presser/releases/latest).
2. Put it in its own folder — `config.json` is created next to it.
3. Run it. There's nothing to install.

> **SmartScreen.** The build isn't code-signed, so Windows may warn on first launch. Choose "More info" → "Run anyway". The source is fully open if you'd rather build it yourself.

### Usage

1. **Target Key** — the key that gets pressed.
2. **Default Frequency (Hz)** — presses per second in the normal state.
3. **RMB Frequency (Hz)** — presses per second while the right mouse button is held.
4. **Toggle Hotkey** — 2–3 keys, at least one modifier.
5. **Start** — saves your settings and minimises to the tray.

From there the app lives in the tray. The hotkey toggles pressing on and off from anywhere. Right-click the tray icon for "Open Settings" and "Exit".

### Configuration

`config.json` is written next to the `.exe` on first launch. You can edit it by hand; a corrupt file falls back to defaults.

```json
{
  "target_key": "E",
  "default_frequency_hz": 4.0,
  "rmb_frequency_hz": 7.0,
  "hotkey": ["Ctrl", "W", "S"]
}
```

| Field | Type | Description |
|---|---|---|
| `target_key` | string | Key to press: `A`–`Z`, `0`–`9`, `F1`–`F24`, `Space`, `Enter`, `Tab`, `Mouse4`, `Mouse5`, and more |
| `default_frequency_hz` | number | Presses per second in the normal state |
| `rmb_frequency_hz` | number | Presses per second while RMB is held |
| `hotkey` | string[] | 2–3 keys, at least one modifier (`Ctrl`, `Alt`, `Shift`, `Win`) |

**Hotkey rules:** 2 to 3 keys, no duplicates, at least one modifier — otherwise the combo would fire during ordinary typing. System shortcuts (`Ctrl+Alt+Delete`, `Alt+Tab`, `Win+R`, `Ctrl+Shift+Esc`, and others) are blocked, including any superset of them.

### Build from source

```bash
git clone https://github.com/Mahiron-hq/driftless-presser.git
cd driftless-presser
cargo build --release
```

The binary lands in `target/release/`. Release profile: `opt-level = 3`, LTO, `codegen-units = 1`, `panic = "abort"`, `strip`.

The platform-independent core (config, key parsing, scheduler math) is unit tested and runs on any OS:

```bash
cargo test --lib
```

### How it works

Three threads with clean boundaries:

- **input-hook** — `WH_KEYBOARD_LL` and `WH_MOUSE_LL` hooks on a dedicated message loop. The callback does as little as possible: Windows drops a hook that's slow to return, stalling input system-wide. The app tags its own `SendInput` output via `dwExtraInfo` so the hook ignores it.
- **clicker** — scheduling and `SendInput`. Blocks on `recv()` while idle and uses `recv_timeout` while active, so toggling off or an RMB change takes effect immediately instead of waiting out the current interval.
- **main/GUI** — settings window and tray, built on `native-windows-gui`.

Presses are sent as scan codes rather than virtual-key codes, which is what engines like SDL2 expect since it matches real hardware. A 30 ms hold separates down from up: some games sample keyboard state once per frame and would miss an instantaneous pair entirely.

### Limitations

- Windows 10/11 only — the app relies on Win32 APIs with no cross-platform equivalent (non-Windows builds stop at a `compile_error!`).
- If the target application runs as administrator, run Driftless Presser as administrator too, or Windows will block synthetic input to its window.
- Anti-cheat systems treat input automation as a violation. Use this in single-player games, in software, and wherever the rules allow it — how you use it is on you.

### License

Apache License 2.0 — see [LICENSE](LICENSE).

---

<div align="center">
<sub>Copyright © 2026 Lev Burmistrov (Mahiron)</sub>
</div>
