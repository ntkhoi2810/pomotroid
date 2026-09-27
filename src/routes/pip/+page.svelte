<script lang="ts">
  import '../../app.css';
  import { onMount } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import {
    getPlants,
    getSettings,
    getThemes,
    getTimerState,
    onPlantsChanged,
    onRoundChange,
    onSettingsChanged,
    onThemesChanged,
    onTimerDurationAdjusted,
    onTimerPaused,
    onTimerReset,
    onTimerResumed,
    onTimerStarted,
    onTimerTick,
    timerRestartRound,
    timerSkip,
    timerToggle,
  } from '$lib/ipc';
  import PlantIllustration from '$lib/components/PlantIllustration.svelte';
  import WeatherScene from '$lib/components/WeatherScene.svelte';
  import { settings } from '$lib/stores/settings';
  import { timerState } from '$lib/stores/timer';
  import { applyTheme } from '$lib/stores/theme';
  import type { MotionActivity, PlantDefinition, WeatherType } from '$lib/types';
  import { resolveThemeName } from '$lib/utils/theme';
  import { setLocale } from '$lib/locale.svelte.js';
  import * as m from '$paraglide/messages.js';

  const WEATHER_TYPES: WeatherType[] = ['sunny', 'rain', 'wind', 'storm'];

  let plants = $state<PlantDefinition[]>([]);
  let weather = $state<WeatherType>('sunny');
  let snapshot = $derived($timerState);
  let displayPlantId = $derived(snapshot.active_plant_id ?? snapshot.selected_plant_id);
  let displayPlant = $derived(plants.find((plant) => plant.id === displayPlantId));
  let growthProgress = $derived(
    snapshot.round_type === 'work' && (snapshot.is_running || snapshot.is_paused)
      ? Math.min(1, snapshot.elapsed_secs / Math.max(1, snapshot.total_secs))
      : 0.18
  );
  let elapsedProgress = $derived(
    Math.min(1, snapshot.elapsed_secs / Math.max(1, snapshot.total_secs))
  );
  let displayIconPath = $derived(
    !displayPlant
      ? null
      : growthProgress < 0.45
        ? displayPlant.small_icon_path
        : growthProgress < 0.78
          ? displayPlant.medium_icon_path
          : displayPlant.large_icon_path
  );
  let motionActivity = $derived<MotionActivity>(
    snapshot.is_running ? 'running' : snapshot.is_paused ? 'paused' : 'idle'
  );

  function randomizeWeather() {
    weather = WEATHER_TYPES[Math.floor(Math.random() * WEATHER_TYPES.length)];
  }

  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  }

  function roundLabel(roundType: string): string {
    if (roundType === 'work') return m.round_label_work();
    if (roundType === 'short-break') return m.round_label_short_break();
    return m.round_label_long_break();
  }

  async function applyCurrentTheme() {
    const themes = await getThemes();
    const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const active =
      themes.find((theme) => theme.name === resolveThemeName($settings, dark)) ?? themes[0];
    if (active) applyTheme(active);
  }

  async function startResize(direction: string) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await getCurrentWebviewWindow().startResizeDragging(direction as any);
  }

  onMount(() => {
    const cleanups: UnlistenFn[] = [];
    (async () => {
      const [currentSettings, currentTimer, catalog] = await Promise.all([
        getSettings(),
        getTimerState(),
        getPlants(),
      ]);
      settings.set(currentSettings);
      timerState.set(currentTimer);
      plants = catalog;
      setLocale(currentSettings.language);
      if (currentTimer.round_type === 'work') randomizeWeather();
      await applyCurrentTheme();

      const colorScheme = window.matchMedia('(prefers-color-scheme: dark)');
      const handleColorSchemeChange = () => {
        if ($settings.theme_mode === 'auto') void applyCurrentTheme();
      };
      colorScheme.addEventListener('change', handleColorSchemeChange);
      cleanups.push(() => colorScheme.removeEventListener('change', handleColorSchemeChange));

      cleanups.push(
        await onTimerStarted((state) => timerState.set(state)),
        await onTimerTick(({ elapsed_secs, total_secs }) => {
          timerState.update((state) => ({
            ...state,
            elapsed_secs,
            total_secs,
            is_running: true,
            is_paused: false,
          }));
        }),
        await onTimerPaused(({ elapsed_secs }) => {
          timerState.update((state) => ({
            ...state,
            elapsed_secs,
            is_running: false,
            is_paused: true,
          }));
        }),
        await onTimerResumed(({ elapsed_secs }) => {
          timerState.update((state) => ({
            ...state,
            elapsed_secs,
            is_running: true,
            is_paused: false,
          }));
        }),
        await onRoundChange((state) => {
          timerState.set(state);
          if (state.round_type === 'work') randomizeWeather();
        }),
        await onTimerReset((state) => timerState.set(state)),
        await onTimerDurationAdjusted((state) => timerState.set(state)),
        await onPlantsChanged((catalog) => (plants = catalog)),
        await onSettingsChanged(async (updated) => {
          settings.set(updated);
          setLocale(updated.language);
          await applyCurrentTheme();
        }),
        await onThemesChanged(async () => applyCurrentTheme())
      );
      await getCurrentWebviewWindow().show();
    })();
    return () => cleanups.forEach((cleanup) => cleanup());
  });
</script>

<main data-tauri-drag-region>
  <section
    class="garden"
    class:resting={snapshot.round_type !== 'work'}
    data-weather={weather}
    data-tauri-drag-region
  >
    <WeatherScene {weather} activity={motionActivity} compact />
    <div class="hill hill-back" data-tauri-drag-region></div>
    <div class="hill hill-front" data-tauri-drag-region></div>

    <header data-tauri-drag-region>
      <div class="copy" data-tauri-drag-region>
        <span data-tauri-drag-region>{roundLabel(snapshot.round_type)}</span>
        <strong data-tauri-drag-region>
          {snapshot.round_type === 'work' && (snapshot.is_running || snapshot.is_paused)
            ? m.plant_growing()
            : m.plant_ready()}
        </strong>
      </div>
    </header>

    <div class="clock" data-tauri-drag-region>
      {formatTime(Math.max(0, snapshot.total_secs - snapshot.elapsed_secs))}
    </div>

    {#if displayPlantId}
      <div class="plant-stage" class:growing={snapshot.is_running} data-tauri-drag-region>
        <PlantIllustration
          plantId={displayPlantId}
          iconPath={displayIconPath}
          progress={growthProgress}
          label={displayPlant?.name}
          category={displayPlant?.category}
          {weather}
          activity={motionActivity}
        />
      </div>
    {/if}

    <div class="soil-line" data-tauri-drag-region></div>
    <div class="progress-track" aria-hidden="true">
      <span style="width: {Math.round(elapsedProgress * 100)}%"></span>
    </div>
  </section>

  <div class="controls" data-tauri-drag-region>
    <button class="side-control" onclick={timerRestartRound} aria-label={m.tooltip_restart_round()}>
      <svg viewBox="0 0 24 24"><path d="M5 5v5h5M5.5 10A7 7 0 1 1 7 18" /></svg>
    </button>
    <button
      class="main-control"
      onclick={timerToggle}
      aria-label={snapshot.is_running ? 'Pause' : 'Start'}
    >
      {#if snapshot.is_running}
        <svg viewBox="0 0 24 24">
          <rect x="6" y="5" width="4" height="14" rx="1" />
          <rect x="14" y="5" width="4" height="14" rx="1" />
        </svg>
      {:else}
        <svg viewBox="0 0 24 24"><path d="m8 5 11 7-11 7Z" /></svg>
      {/if}
    </button>
    <button class="side-control" onclick={timerSkip} aria-label={m.tooltip_skip()}>
      <svg viewBox="0 0 24 24"><path d="m5 5 9 7-9 7ZM17 5v14" /></svg>
    </button>
  </div>

  <button
    class="close"
    onclick={() => getCurrentWebviewWindow().close()}
    aria-label="Close mini timer"
  >
    <svg viewBox="0 0 16 16">
      <path d="m4 4 8 8M12 4l-8 8" />
    </svg>
  </button>
</main>

<div class="resize-handle north" onmousedown={() => startResize('North')} role="none"></div>
<div class="resize-handle south" onmousedown={() => startResize('South')} role="none"></div>
<div class="resize-handle east" onmousedown={() => startResize('East')} role="none"></div>
<div class="resize-handle west" onmousedown={() => startResize('West')} role="none"></div>
<div
  class="resize-handle north-east"
  onmousedown={() => startResize('NorthEast')}
  role="none"
></div>
<div
  class="resize-handle north-west"
  onmousedown={() => startResize('NorthWest')}
  role="none"
></div>
<div
  class="resize-handle south-east"
  onmousedown={() => startResize('SouthEast')}
  role="none"
></div>
<div
  class="resize-handle south-west"
  onmousedown={() => startResize('SouthWest')}
  role="none"
></div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
  }

  main {
    position: relative;
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--color-foreground);
    border: 1px solid color-mix(in oklch, var(--color-foreground) 10%, transparent);
    border-radius: clamp(16px, 7vw, 28px);
    background: color-mix(in oklch, var(--color-background-light) 62%, var(--color-background));
    box-shadow:
      0 18px 45px color-mix(in oklch, #000 24%, transparent),
      inset 0 1px 0 color-mix(in oklch, var(--color-foreground) 9%, transparent);
  }

  .garden {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: linear-gradient(
      155deg,
      color-mix(in oklch, var(--color-short-round) 38%, var(--color-background)) 0%,
      color-mix(in oklch, var(--color-accent) 18%, var(--color-background-light)) 100%
    );
    transition: background 0.8s ease;
  }

  .garden.resting {
    background: linear-gradient(
      160deg,
      color-mix(in oklch, var(--color-long-round) 25%, var(--color-background)) 0%,
      var(--color-background-light) 100%
    );
  }

  header {
    position: relative;
    z-index: 4;
    padding: clamp(15px, 6vw, 24px) clamp(16px, 7vw, 28px) 0;
    pointer-events: none;
  }

  .copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .copy span {
    color: color-mix(in oklch, var(--color-foreground) 72%, transparent);
    font-size: clamp(9px, 3.5vw, 12px);
    font-weight: 780;
    letter-spacing: 0.15em;
    text-transform: uppercase;
  }

  .copy strong {
    max-width: calc(100% - 34px);
    overflow: hidden;
    font-size: clamp(12px, 4.5vw, 17px);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .clock {
    position: absolute;
    z-index: 4;
    top: clamp(62px, 24%, 102px);
    left: 50%;
    transform: translateX(-50%);
    font:
      670 clamp(27px, 12vw, 46px) / 1 'Mona Sans Mono',
      monospace;
    letter-spacing: 0.035em;
    text-shadow: 0 2px 14px color-mix(in oklch, var(--color-background) 50%, transparent);
    white-space: nowrap;
    pointer-events: none;
  }

  .hill {
    position: absolute;
    z-index: 1;
    bottom: -16%;
    border-radius: 52% 62% 0 0 / 68% 74% 0 0;
    transition: background 0.8s ease;
  }

  .hill-back {
    width: 112%;
    height: 42%;
    left: -40%;
    background: color-mix(in oklch, var(--color-short-round) 52%, var(--color-background));
    transform: rotate(4deg);
  }

  .hill-front {
    width: 122%;
    height: 37%;
    right: -45%;
    background: color-mix(in oklch, var(--color-short-round) 68%, var(--color-background));
    transform: rotate(-3deg);
  }

  .garden[data-weather='rain'] .hill,
  .garden[data-weather='storm'] .hill {
    filter: saturate(0.78) brightness(0.86);
  }

  .plant-stage {
    position: absolute;
    z-index: 3;
    width: clamp(128px, 60vw, 230px);
    height: clamp(128px, 60vw, 230px);
    left: 50%;
    bottom: clamp(4px, 2vh, 14px);
    transform: translateX(-50%);
    pointer-events: none;
  }

  .soil-line {
    position: absolute;
    z-index: 2;
    width: min(66%, 240px);
    height: clamp(20px, 8%, 34px);
    left: 50%;
    bottom: clamp(4px, 2vh, 12px);
    transform: translateX(-50%);
    border-radius: 50%;
    background: radial-gradient(
      ellipse at center,
      color-mix(in oklch, #8b654f 68%, var(--color-background)) 0%,
      color-mix(in oklch, #5d4638 48%, transparent) 72%
    );
  }

  .progress-track {
    position: absolute;
    z-index: 5;
    right: 0;
    bottom: 0;
    left: 0;
    height: 4px;
    background: color-mix(in oklch, var(--color-background) 30%, transparent);
  }

  .progress-track span {
    display: block;
    height: 100%;
    background: color-mix(in oklch, var(--color-accent) 75%, #fff);
    transition: width 1s linear;
  }

  .controls {
    flex: 0 0 clamp(66px, 22vh, 92px);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: clamp(9px, 4vw, 18px);
    background: linear-gradient(
      155deg,
      color-mix(in oklch, var(--color-background-light) 82%, transparent),
      color-mix(in oklch, var(--color-background) 68%, transparent)
    );
  }

  button {
    display: grid;
    place-items: center;
    border: 0;
    color: inherit;
    cursor: pointer;
  }

  .controls button {
    flex: 0 0 auto;
    border-radius: 50%;
    transition:
      transform var(--transition-snappy),
      box-shadow var(--transition-default),
      background var(--transition-default);
  }

  .controls button:hover {
    transform: translateY(-1px);
  }

  .controls button:active {
    transform: scale(0.96);
  }

  .controls svg {
    width: clamp(17px, 6vw, 23px);
    height: clamp(17px, 6vw, 23px);
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .main-control {
    width: clamp(45px, 17vw, 62px);
    height: clamp(45px, 17vw, 62px);
    color: var(--color-background);
    background: var(--color-accent);
    box-shadow:
      0 7px 19px color-mix(in oklch, var(--color-accent) 32%, transparent),
      inset 0 1px 0 color-mix(in oklch, #fff 24%, transparent);
  }

  .main-control svg {
    fill: currentColor;
    stroke: none;
  }

  .side-control {
    width: clamp(36px, 13vw, 47px);
    height: clamp(36px, 13vw, 47px);
    background: var(--color-hover);
  }

  button:focus-visible {
    outline: 2px solid color-mix(in oklch, var(--color-accent) 78%, #fff);
    outline-offset: 2px;
  }

  .close {
    position: absolute;
    z-index: 8;
    top: 9px;
    right: 9px;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    opacity: 0;
    color: var(--color-foreground);
    background: color-mix(in oklch, var(--color-background) 45%, transparent);
    backdrop-filter: blur(5px);
    transform: translateY(-3px);
    transition:
      opacity var(--transition-default),
      transform var(--transition-default),
      background var(--transition-default);
  }

  main:hover .close,
  main:focus-within .close,
  .close:focus-visible {
    opacity: 1;
    transform: translateY(0);
  }

  .close:hover {
    background: var(--color-focus-round);
  }

  .close svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
  }

  .resize-handle {
    position: fixed;
    z-index: 20;
  }

  .north,
  .south {
    right: 8px;
    left: 8px;
    height: 5px;
  }

  .north {
    top: 0;
    cursor: n-resize;
  }

  .south {
    bottom: 0;
    cursor: s-resize;
  }

  .east,
  .west {
    top: 8px;
    bottom: 8px;
    width: 5px;
  }

  .east {
    right: 0;
    cursor: e-resize;
  }

  .west {
    left: 0;
    cursor: w-resize;
  }

  .north-east,
  .north-west,
  .south-east,
  .south-west {
    width: 12px;
    height: 12px;
  }

  .north-east {
    top: 0;
    right: 0;
    cursor: ne-resize;
  }

  .north-west {
    top: 0;
    left: 0;
    cursor: nw-resize;
  }

  .south-east {
    right: 0;
    bottom: 0;
    cursor: se-resize;
  }

  .south-west {
    bottom: 0;
    left: 0;
    cursor: sw-resize;
  }

  @media (max-width: 250px), (max-height: 300px) {
    .copy strong {
      display: none;
    }

    .clock {
      top: clamp(47px, 21%, 62px);
    }

    .plant-stage {
      width: clamp(105px, 52vw, 135px);
      height: clamp(105px, 52vw, 135px);
    }

    .controls {
      flex-basis: 64px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .garden,
    .hill,
    .progress-track span,
    .controls button,
    .close {
      transition-duration: 0.01ms !important;
    }
  }
</style>
