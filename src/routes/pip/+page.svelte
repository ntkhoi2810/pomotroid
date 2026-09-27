<script lang="ts">
  import '../../app.css';
  import { onMount } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import {
    getSettings,
    getThemes,
    getTimerState,
    onRoundChange,
    onSettingsChanged,
    onThemesChanged,
    onTimerDurationAdjusted,
    onTimerPaused,
    onTimerReset,
    onTimerResumed,
    onTimerStarted,
    onTimerTick,
    timerToggle,
  } from '$lib/ipc';
  import { settings } from '$lib/stores/settings';
  import { timerState } from '$lib/stores/timer';
  import { applyTheme } from '$lib/stores/theme';
  import { resolveThemeName } from '$lib/utils/theme';
  import { setLocale } from '$lib/locale.svelte.js';
  import * as m from '$paraglide/messages.js';

  let snapshot = $derived($timerState);

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

  onMount(() => {
    const cleanups: UnlistenFn[] = [];
    (async () => {
      const [currentSettings, currentTimer] = await Promise.all([getSettings(), getTimerState()]);
      settings.set(currentSettings);
      timerState.set(currentTimer);
      setLocale(currentSettings.language);
      await applyCurrentTheme();

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
        await onRoundChange((state) => timerState.set(state)),
        await onTimerReset((state) => timerState.set(state)),
        await onTimerDurationAdjusted((state) => timerState.set(state)),
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
  <div class="copy" data-tauri-drag-region>
    <span>{roundLabel(snapshot.round_type)}</span>
    <strong>{formatTime(Math.max(0, snapshot.total_secs - snapshot.elapsed_secs))}</strong>
  </div>
  <button class="toggle" onclick={timerToggle} aria-label={snapshot.is_running ? 'Pause' : 'Start'}>
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
  <button class="close" onclick={() => getCurrentWebviewWindow().close()} aria-label="Close"
    >x</button
  >
  <span
    class="progress"
    style="width: {Math.min(
      100,
      (snapshot.elapsed_secs / Math.max(1, snapshot.total_secs)) * 100
    )}%"
  ></span>
</main>

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
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    overflow: hidden;
    color: var(--color-foreground);
    background: color-mix(in oklch, var(--color-background-light) 94%, transparent);
    border: 1px solid var(--color-separator);
  }
  .copy {
    flex: 1;
    display: flex;
    flex-direction: column;
    pointer-events: none;
  }
  .copy span {
    color: var(--color-foreground-darker);
    font-size: 10px;
    font-weight: 750;
    letter-spacing: 0.13em;
    text-transform: uppercase;
  }
  .copy strong {
    font:
      650 30px/1.2 'Mona Sans Mono',
      monospace;
    letter-spacing: 0.04em;
  }
  button {
    border: 0;
    color: inherit;
    cursor: pointer;
  }
  .toggle {
    width: 42px;
    height: 42px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--color-background);
    background: var(--color-accent);
  }
  .toggle svg {
    width: 20px;
    fill: currentColor;
  }
  .close {
    position: absolute;
    top: 4px;
    right: 6px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    color: var(--color-foreground-darker);
    background: transparent;
    font-size: 13px;
  }
  .progress {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 3px;
    background: var(--color-accent);
    transition: width 1s linear;
  }
</style>
