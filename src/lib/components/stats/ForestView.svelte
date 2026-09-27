<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getForest,
    getPlants,
    onPlantsChanged,
    onRoundChange,
    onSessionsCleared,
  } from '$lib/ipc';
  import type { ForestData, ForestPeriod, PlantDefinition } from '$lib/types';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import * as m from '$paraglide/messages.js';
  import PlantIllustration from '$lib/components/PlantIllustration.svelte';

  let period = $state<ForestPeriod>('day');
  let anchor = $state(new Date());
  let forest = $state<ForestData | null>(null);
  let plants = $state<PlantDefinition[]>([]);
  let loading = $state(true);

  function toLocalDate(date: Date): string {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  function parseLocalDate(value: string): Date {
    const [year, month, day] = value.split('-').map(Number);
    return new Date(year, month - 1, day);
  }

  async function load() {
    loading = true;
    try {
      forest = await getForest(period, toLocalDate(anchor));
    } finally {
      loading = false;
    }
  }

  async function setPeriod(next: ForestPeriod) {
    period = next;
    anchor = new Date();
    await load();
  }

  async function move(direction: -1 | 1) {
    const next = new Date(anchor);
    if (period === 'day') next.setDate(next.getDate() + direction);
    if (period === 'week') next.setDate(next.getDate() + direction * 7);
    if (period === 'month') next.setMonth(next.getMonth() + direction);
    anchor = next;
    await load();
  }

  async function today() {
    anchor = new Date();
    await load();
  }

  function rangeLabel(data: ForestData | null): string {
    if (!data) return '';
    const start = parseLocalDate(data.start_date);
    const end = parseLocalDate(data.end_date);
    end.setDate(end.getDate() - 1);
    const format = new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
    if (data.start_date === toLocalDate(end)) return format.format(start);
    return `${format.format(start)} – ${format.format(end)}`;
  }

  function durationLabel(seconds: number): string {
    const mins = Math.round(seconds / 60);
    return mins < 60 ? `${mins}m` : `${Math.floor(mins / 60)}h ${mins % 60}m`;
  }

  function plantName(id: string, snapshot: string | null): string {
    return snapshot ?? plants.find((plant) => plant.id === id)?.name ?? id;
  }

  onMount(() => {
    const cleanups: UnlistenFn[] = [];
    (async () => {
      plants = await getPlants(true);
      await load();
      cleanups.push(
        await onRoundChange(load),
        await onSessionsCleared(load),
        await onPlantsChanged(async () => {
          plants = await getPlants(true);
        })
      );
    })();
    return () => cleanups.forEach((cleanup) => cleanup());
  });
</script>

<div class="forest-view">
  <header>
    <div class="period-tabs">
      <button class:active={period === 'day'} onclick={() => setPeriod('day')}
        >{m.forest_day()}</button
      >
      <button class:active={period === 'week'} onclick={() => setPeriod('week')}
        >{m.forest_week()}</button
      >
      <button class:active={period === 'month'} onclick={() => setPeriod('month')}
        >{m.forest_month()}</button
      >
    </div>
    <div class="navigation">
      <button onclick={() => move(-1)} aria-label={m.forest_previous()}>‹</button>
      <button class="range" onclick={today} title={m.forest_today()}>{rangeLabel(forest)}</button>
      <button onclick={() => move(1)} aria-label={m.forest_next()}>›</button>
    </div>
    <div class="forest-summary">
      <strong>{forest?.entries.length ?? 0}</strong><span>{m.forest_trees()}</span>
      <i></i>
      <strong>{durationLabel(forest?.total_focus_secs ?? 0)}</strong><span>{m.forest_focus()}</span>
    </div>
  </header>

  <div class="forest-scene" class:loading>
    <div class="sun"></div>
    <div class="ridge back"></div>
    <div class="ridge front"></div>
    {#if !loading && forest?.entries.length === 0}
      <div class="empty">
        <span>⌁</span>
        <p>{m.forest_empty()}</p>
      </div>
    {:else}
      <div class="trees">
        {#each forest?.entries ?? [] as entry, index (entry.session_id)}
          <div
            class="tree"
            style="--delay: {Math.min(index * 25, 350)}ms"
            title={`${plantName(entry.plant_id, entry.plant_name)} · ${durationLabel(entry.duration_secs)}`}
          >
            <PlantIllustration
              plantId={entry.plant_id}
              iconPath={entry.icon_path}
              stage={entry.growth_stage}
            />
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .forest-view {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--color-background);
    animation: app-fade-in 0.2s ease;
  }
  header {
    min-height: 82px;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    padding: 12px 24px;
    border-bottom: 1px solid var(--color-separator);
    gap: 16px;
  }
  .period-tabs {
    display: flex;
    justify-self: start;
    padding: 3px;
    border-radius: 9px;
    background: var(--color-hover);
  }
  .period-tabs button {
    border: 0;
    border-radius: 7px;
    padding: 7px 12px;
    background: transparent;
    color: var(--color-foreground-darker);
    font: 650 10px/1 'Mona Sans';
    text-transform: uppercase;
    letter-spacing: 0.07em;
    cursor: pointer;
  }
  .period-tabs button.active {
    background: var(--color-background-light);
    color: var(--color-foreground);
    box-shadow: 0 2px 7px color-mix(in oklch, #000 15%, transparent);
  }
  .navigation {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
  }
  .navigation button {
    border: 0;
    background: none;
    color: var(--color-foreground-darker);
    cursor: pointer;
    font-size: 20px;
  }
  .navigation .range {
    min-width: 180px;
    color: var(--color-foreground);
    font: 620 12px/1 'Mona Sans';
  }
  .forest-summary {
    justify-self: end;
    display: grid;
    grid-template-columns: auto auto;
    gap: 1px 5px;
    align-items: baseline;
  }
  .forest-summary strong {
    font: 700 15px/1 'Mona Sans Mono';
  }
  .forest-summary span {
    color: var(--color-foreground-darker);
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .forest-summary i {
    grid-column: 1 / -1;
    height: 4px;
  }
  .forest-scene {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: linear-gradient(
      180deg,
      color-mix(in oklch, var(--color-short-round) 19%, var(--color-background)) 0%,
      color-mix(in oklch, var(--color-background-light) 70%, var(--color-background)) 65%
    );
    transition: opacity 0.15s;
  }
  .forest-scene.loading {
    opacity: 0.65;
  }
  .sun {
    position: absolute;
    right: 70px;
    top: 24px;
    width: 75px;
    height: 75px;
    border-radius: 50%;
    background: color-mix(in oklch, #f5d77a 35%, transparent);
    box-shadow: 0 0 45px color-mix(in oklch, #f5d77a 20%, transparent);
  }
  .ridge {
    position: absolute;
    width: 70%;
    height: 160px;
    bottom: -80px;
    border-radius: 50% 50% 0 0;
    pointer-events: none;
  }
  .ridge.back {
    left: -12%;
    background: color-mix(in oklch, var(--color-short-round) 35%, var(--color-background));
  }
  .ridge.front {
    right: -18%;
    height: 190px;
    background: color-mix(in oklch, var(--color-short-round) 47%, var(--color-background));
  }
  .trees {
    position: relative;
    z-index: 2;
    min-height: 100%;
    padding: 28px 36px 38px;
    display: grid;
    grid-template-columns: repeat(8, minmax(70px, 1fr));
    grid-auto-rows: 105px;
    align-items: end;
    gap: 0 8px;
  }
  .tree {
    height: 105px;
    animation: tree-in 0.35s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: var(--delay);
    filter: drop-shadow(0 5px 5px color-mix(in oklch, #000 12%, transparent));
  }
  @keyframes tree-in {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.85);
    }
  }
  .empty {
    position: relative;
    z-index: 3;
    height: 100%;
    min-height: 280px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--color-foreground-darker);
    gap: 8px;
  }
  .empty span {
    font-size: 48px;
    color: var(--color-short-round);
    transform: rotate(-10deg);
  }
  .empty p {
    max-width: 300px;
    text-align: center;
    font-size: 13px;
    line-height: 1.5;
  }
  @media (max-width: 700px) {
    header {
      grid-template-columns: 1fr 1fr;
    }
    .navigation {
      grid-column: 1 / -1;
      grid-row: 2;
    }
    .trees {
      grid-template-columns: repeat(5, 1fr);
    }
    .forest-summary {
      justify-self: end;
    }
  }
</style>
