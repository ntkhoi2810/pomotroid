<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getPlants,
    getTimerState,
    notificationShow,
    onRoundChange,
    onTimerPaused,
    onTimerReset,
    onTimerResumed,
    onTimerStarted,
    onTimerTick,
    selectPlant,
    timerReset,
    timerRestartRound,
    timerSkip,
    timerToggle,
  } from '$lib/ipc';
  import { timerState } from '$lib/stores/timer';
  import { settings } from '$lib/stores/settings';
  import type { PlantDefinition } from '$lib/types';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import * as m from '$paraglide/messages.js';
  import PlantIllustration from './PlantIllustration.svelte';

  interface Props {
    isCompact?: boolean;
    uiScale?: number;
  }

  let { isCompact = false, uiScale = 1 }: Props = $props();
  let plants = $state<PlantDefinition[]>([]);
  let showPicker = $state(false);
  let showClock = $state(true);
  let selecting = $state(false);
  let snapshot = $derived($timerState);
  let selectedPlant = $derived(plants.find((plant) => plant.id === snapshot.selected_plant_id));
  let displayPlantId = $derived(snapshot.active_plant_id ?? snapshot.selected_plant_id);
  let displayPlant = $derived(plants.find((plant) => plant.id === displayPlantId));
  let progress = $derived(
    snapshot.round_type === 'work' && (snapshot.is_running || snapshot.is_paused)
      ? Math.min(1, snapshot.elapsed_secs / Math.max(1, snapshot.total_secs))
      : 0.18
  );
  let isGrowing = $derived(
    snapshot.round_type === 'work' && (snapshot.is_running || snapshot.is_paused)
  );
  let selectingNext = $derived(Boolean(snapshot.active_plant_id));

  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  }

  function minimumLabel(seconds: number): string {
    return `${Math.ceil(seconds / 60)} min`;
  }

  function roundLabel(roundType: string): string {
    if (roundType === 'work') return m.round_label_work();
    if (roundType === 'short-break') return m.round_label_short_break();
    return m.round_label_long_break();
  }

  async function choosePlant(plant: PlantDefinition) {
    if (plant.min_focus_secs > $settings.time_work_secs || selecting) return;
    selecting = true;
    try {
      timerState.set(await selectPlant(plant.id));
      showPicker = false;
    } finally {
      selecting = false;
    }
  }

  onMount(() => {
    const cleanups: UnlistenFn[] = [];
    (async () => {
      const [initial, catalog] = await Promise.all([getTimerState(), getPlants()]);
      timerState.set(initial);
      plants = catalog;

      cleanups.push(
        await onTimerStarted((snapshot) => timerState.set(snapshot)),
        await onTimerTick(({ elapsed_secs, total_secs }) => {
          timerState.update((current) => ({
            ...current,
            elapsed_secs,
            total_secs,
            is_running: true,
            is_paused: false,
          }));
        }),
        await onTimerPaused(({ elapsed_secs }) => {
          timerState.update((current) => ({
            ...current,
            elapsed_secs,
            is_running: false,
            is_paused: true,
          }));
        }),
        await onTimerResumed(({ elapsed_secs }) => {
          timerState.update((current) => ({
            ...current,
            elapsed_secs,
            is_running: true,
            is_paused: false,
          }));
        }),
        await onRoundChange((snapshot) => {
          timerState.set(snapshot);
          if (!$settings.notifications_enabled) return;
          if (snapshot.round_type === 'work') {
            const afterBreak = ['short-break', 'long-break'].includes(snapshot.previous_round_type);
            notificationShow(
              afterBreak ? m.notification_work_title() : m.notification_work_start_title(),
              afterBreak ? m.notification_work_body() : m.notification_work_start_body()
            ).catch(() => {});
          } else if (snapshot.round_type === 'short-break') {
            notificationShow(
              m.notification_short_break_title(),
              m.notification_short_break_body()
            ).catch(() => {});
          } else {
            notificationShow(
              m.notification_long_break_title(),
              m.notification_long_break_body()
            ).catch(() => {});
          }
        }),
        await onTimerReset((snapshot) => timerState.set(snapshot))
      );
    })();

    return () => cleanups.forEach((cleanup) => cleanup());
  });
</script>

<div class="timer-shell" class:compact={isCompact} style="zoom: {uiScale}">
  <section class="garden" class:resting={snapshot.round_type !== 'work'}>
    <div class="sky-glow"></div>
    <div class="cloud cloud-one"></div>
    <div class="cloud cloud-two"></div>
    <div class="hill hill-back"></div>
    <div class="hill hill-front"></div>

    <header class="garden-header">
      <div>
        <span class="eyebrow">{roundLabel(snapshot.round_type)}</span>
        <strong>{isGrowing ? m.plant_growing() : m.plant_ready()}</strong>
      </div>
      <button
        class="icon-button"
        onclick={() => (showClock = !showClock)}
        aria-label={m.plant_toggle_clock()}
      >
        {#if showClock}
          <svg viewBox="0 0 24 24"
            ><path d="M4 12s3-6 8-6 8 6 8 6-3 6-8 6-8-6-8-6Z" /><circle
              cx="12"
              cy="12"
              r="2.5"
            /></svg
          >
        {:else}
          <svg viewBox="0 0 24 24"
            ><path
              d="M3 3l18 18M5 8c-1 1-2 2.5-2 4 0 0 3 6 9 6 1.5 0 2.8-.4 4-1M9 6.4A8 8 0 0 1 12 6c6 0 9 6 9 6a11 11 0 0 1-2 3"
            /></svg
          >
        {/if}
      </button>
    </header>

    {#if showClock}
      <button class="clock" onclick={() => (showClock = false)} aria-label={m.plant_hide_clock()}>
        {formatTime(Math.max(0, snapshot.total_secs - snapshot.elapsed_secs))}
      </button>
    {/if}

    <div class="plant-stage" class:growing={snapshot.is_running}>
      <PlantIllustration plantId={displayPlantId} {progress} label={displayPlant?.name} />
    </div>

    <div class="soil-line"></div>
    <div class="progress-track">
      <span style="width: {Math.round(progress * 100)}%"></span>
    </div>
  </section>

  <div class="plant-meta">
    <button class="plant-choice" onclick={() => (showPicker = true)}>
      <span class="seed-dot" style="--seed-color: {selectedPlant?.accent ?? '#77b255'}"></span>
      <span>
        <small>{selectingNext ? m.plant_next_label() : m.plant_selected_label()}</small>
        <strong>{selectedPlant?.name ?? m.plant_loading()}</strong>
      </span>
      <svg viewBox="0 0 24 24"><path d="m8 10 4 4 4-4" /></svg>
    </button>

    <div class="controls">
      <button
        class="side-control"
        onclick={timerRestartRound}
        aria-label={m.tooltip_restart_round()}
      >
        <svg viewBox="0 0 24 24"><path d="M5 5v5h5M5.5 10A7 7 0 1 1 7 18" /></svg>
      </button>
      <button
        class="main-control"
        onclick={timerToggle}
        aria-label={snapshot.is_running ? 'Pause' : 'Start'}
      >
        {#if snapshot.is_running}
          <svg viewBox="0 0 24 24"
            ><rect x="6" y="5" width="4" height="14" rx="1" /><rect
              x="14"
              y="5"
              width="4"
              height="14"
              rx="1"
            /></svg
          >
        {:else}
          <svg viewBox="0 0 24 24"><path d="m8 5 11 7-11 7Z" /></svg>
        {/if}
      </button>
      <button class="side-control" onclick={timerSkip} aria-label={m.tooltip_skip()}>
        <svg viewBox="0 0 24 24"><path d="m5 5 9 7-9 7ZM17 5v14" /></svg>
      </button>
    </div>

    {#if !isCompact}
      <footer>
        <span>{snapshot.work_round_number}/{snapshot.work_rounds_total}</span>
        <button onclick={timerReset}>{m.timer_reset()}</button>
      </footer>
    {/if}
  </div>

  {#if showPicker}
    <div class="picker-backdrop" role="presentation" onclick={() => (showPicker = false)}></div>
    <section class="picker" aria-label={m.plant_picker_title()}>
      <div class="picker-heading">
        <div>
          <small>{selectingNext ? m.plant_choose_next() : m.plant_choose()}</small>
          <h2>{m.plant_picker_title()}</h2>
        </div>
        <button onclick={() => (showPicker = false)} aria-label="Close">×</button>
      </div>
      <div class="plant-grid">
        {#each plants as plant}
          {@const locked = plant.min_focus_secs > $settings.time_work_secs}
          <button
            class:selected={plant.id === snapshot.selected_plant_id}
            class:locked
            disabled={locked || selecting}
            onclick={() => choosePlant(plant)}
          >
            <span class="plant-thumb"><PlantIllustration plantId={plant.id} progress={1} /></span>
            <strong>{plant.name}</strong>
            <small
              >{locked
                ? m.plant_needs_time({ minutes: Math.ceil(plant.min_focus_secs / 60) })
                : minimumLabel(plant.min_focus_secs)}</small
            >
          </button>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .timer-shell {
    position: relative;
    width: 320px;
    height: 412px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 24px;
    background: color-mix(in oklch, var(--color-background-light) 55%, var(--color-background));
    box-shadow: 0 18px 55px color-mix(in oklch, #000 25%, transparent);
  }
  .garden {
    position: relative;
    height: 292px;
    overflow: hidden;
    background: linear-gradient(
      160deg,
      color-mix(in oklch, var(--color-short-round) 38%, var(--color-background)) 0%,
      color-mix(in oklch, var(--color-accent) 18%, var(--color-background-light)) 100%
    );
  }
  .garden.resting {
    background: linear-gradient(
      160deg,
      color-mix(in oklch, var(--color-long-round) 25%, var(--color-background)) 0%,
      var(--color-background-light) 100%
    );
  }
  .sky-glow {
    position: absolute;
    width: 150px;
    height: 150px;
    right: -45px;
    top: -55px;
    border-radius: 50%;
    background: color-mix(in oklch, #fff7c2 22%, transparent);
    filter: blur(2px);
  }
  .cloud {
    position: absolute;
    width: 45px;
    height: 10px;
    border-radius: 20px;
    background: color-mix(in oklch, var(--color-foreground) 15%, transparent);
  }
  .cloud::before,
  .cloud::after {
    content: '';
    position: absolute;
    border-radius: 50%;
    background: inherit;
  }
  .cloud::before {
    width: 18px;
    height: 18px;
    left: 8px;
    bottom: 0;
  }
  .cloud::after {
    width: 13px;
    height: 13px;
    left: 24px;
    bottom: 0;
  }
  .cloud-one {
    top: 65px;
    left: 25px;
  }
  .cloud-two {
    top: 92px;
    right: 30px;
    transform: scale(0.7);
  }
  .hill {
    position: absolute;
    border-radius: 50% 50% 0 0;
    bottom: -55px;
  }
  .hill-back {
    width: 330px;
    height: 135px;
    left: -110px;
    background: color-mix(in oklch, var(--color-short-round) 52%, var(--color-background));
    transform: rotate(5deg);
  }
  .hill-front {
    width: 360px;
    height: 120px;
    right: -130px;
    background: color-mix(in oklch, var(--color-short-round) 68%, var(--color-background));
    transform: rotate(-4deg);
  }
  .garden-header {
    position: relative;
    z-index: 4;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 18px 18px 0;
  }
  .garden-header div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .eyebrow {
    font-size: 9px;
    font-weight: 750;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    opacity: 0.65;
  }
  .garden-header strong {
    font-size: 14px;
  }
  .icon-button {
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: inherit;
    background: color-mix(in oklch, var(--color-background) 28%, transparent);
    cursor: pointer;
  }
  .icon-button svg {
    width: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
  }
  .clock {
    position: absolute;
    z-index: 4;
    left: 50%;
    top: 56px;
    transform: translateX(-50%);
    border: 0;
    background: none;
    color: inherit;
    font:
      650 27px/1 'Mona Sans Mono',
      monospace;
    letter-spacing: 0.04em;
    cursor: pointer;
    text-shadow: 0 2px 14px color-mix(in oklch, var(--color-background) 50%, transparent);
  }
  .plant-stage {
    position: absolute;
    z-index: 3;
    width: 180px;
    height: 180px;
    left: 50%;
    bottom: 19px;
    transform: translateX(-50%);
  }
  .plant-stage.growing {
    animation: breathe 3s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      transform: translateX(-50%) translateY(-2px);
    }
  }
  .soil-line {
    position: absolute;
    z-index: 2;
    height: 28px;
    width: 190px;
    left: 50%;
    bottom: 8px;
    transform: translateX(-50%);
    border-radius: 50%;
    background: color-mix(in oklch, #6b4933 52%, var(--color-background));
  }
  .progress-track {
    position: absolute;
    z-index: 5;
    bottom: 0;
    left: 0;
    width: 100%;
    height: 4px;
    background: color-mix(in oklch, var(--color-background) 30%, transparent);
  }
  .progress-track span {
    display: block;
    height: 100%;
    background: color-mix(in oklch, var(--color-accent) 75%, #fff);
    transition: width 1s linear;
  }
  .plant-meta {
    flex: 1;
    padding: 10px 14px 8px;
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 5px 8px;
  }
  .plant-choice {
    min-width: 0;
    border: 0;
    background: transparent;
    color: inherit;
    display: flex;
    align-items: center;
    gap: 9px;
    text-align: left;
    cursor: pointer;
  }
  .plant-choice > span:nth-child(2) {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .plant-choice small {
    color: var(--color-foreground-darker);
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }
  .plant-choice strong {
    max-width: 115px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
  }
  .plant-choice svg {
    width: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }
  .seed-dot {
    flex: 0 0 auto;
    width: 25px;
    height: 25px;
    border-radius: 9px 9px 12px 12px;
    background: var(--seed-color);
    box-shadow: inset -5px -5px 0 color-mix(in oklch, #000 12%, transparent);
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .controls button {
    border: 0;
    display: grid;
    place-items: center;
    cursor: pointer;
    color: inherit;
  }
  .controls svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .main-control {
    width: 43px;
    height: 43px;
    border-radius: 50%;
    background: var(--color-accent);
    color: var(--color-background) !important;
    box-shadow: 0 5px 16px color-mix(in oklch, var(--color-accent) 35%, transparent);
  }
  .main-control svg {
    fill: currentColor;
    stroke: none;
  }
  .side-control {
    width: 29px;
    height: 29px;
    border-radius: 50%;
    background: var(--color-hover);
  }
  footer {
    grid-column: 1 / -1;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--color-foreground-darker);
    font-size: 9px;
  }
  .timer-shell footer button {
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .picker-backdrop {
    position: absolute;
    inset: 0;
    z-index: 9;
    background: color-mix(in oklch, #000 38%, transparent);
    backdrop-filter: blur(2px);
  }
  .picker {
    position: absolute;
    z-index: 10;
    inset: 64px 9px 9px;
    padding: 14px;
    border-radius: 20px;
    background: color-mix(in oklch, var(--color-background-light) 92%, #000);
    box-shadow: 0 18px 45px color-mix(in oklch, #000 35%, transparent);
    animation: picker-in 0.2s ease-out;
  }
  @keyframes picker-in {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
  }
  .picker-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .picker-heading small {
    color: var(--color-foreground-darker);
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }
  .picker-heading h2 {
    font-size: 17px;
  }
  .picker-heading button {
    border: 0;
    background: var(--color-hover);
    color: inherit;
    width: 27px;
    height: 27px;
    border-radius: 50%;
    font-size: 18px;
    cursor: pointer;
  }
  .plant-grid {
    height: 275px;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 7px;
    padding-right: 2px;
  }
  .plant-grid button {
    min-height: 102px;
    position: relative;
    border: 1px solid var(--color-separator);
    border-radius: 13px;
    background: color-mix(in oklch, var(--color-background) 45%, transparent);
    color: inherit;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 5px;
    cursor: pointer;
  }
  .plant-grid button.selected {
    border-color: var(--color-accent);
    box-shadow: inset 0 0 0 1px var(--color-accent);
  }
  .plant-grid button.locked {
    opacity: 0.42;
    cursor: not-allowed;
  }
  .plant-grid strong {
    font-size: 10px;
  }
  .plant-grid small {
    font-size: 8px;
    color: var(--color-foreground-darker);
  }
  .plant-thumb {
    width: 62px;
    height: 62px;
    margin-bottom: -2px;
  }
  .compact {
    width: 220px;
    height: 220px;
    border-radius: 18px;
  }
  .compact .garden {
    height: 170px;
  }
  .compact .garden-header,
  .compact .clock,
  .compact .plant-choice,
  .compact footer {
    display: none;
  }
  .compact .plant-stage {
    width: 130px;
    height: 130px;
    bottom: 5px;
  }
  .compact .plant-meta {
    display: flex;
    justify-content: center;
    padding: 3px;
  }
  .compact .picker {
    inset: 5px;
  }
  .compact .plant-grid {
    height: 150px;
  }
</style>
