<script lang="ts">
  import {
    deletePlant,
    getPlants,
    openPlantImagePicker,
    restorePlantDefault,
    savePlant,
    setPlantHidden,
  } from '$lib/ipc';
  import type { PlantDefinition, PlantInput } from '$lib/types';
  import PlantIllustration from './PlantIllustration.svelte';
  import * as m from '$paraglide/messages.js';

  interface Props {
    onclose: () => void;
    activePlantId?: string | null;
  }
  let { onclose, activePlantId = null }: Props = $props();
  let plants = $state<PlantDefinition[]>([]);
  let editing = $state<PlantDefinition | null>(null);
  let creating = $state(false);
  let name = $state('');
  let category = $state('flower');
  let minutes = $state(25);
  let accent = $state('#65a653');
  let smallSource = $state<string | null>(null);
  let mediumSource = $state<string | null>(null);
  let largeSource = $state<string | null>(null);
  let saving = $state(false);
  let error = $state('');

  async function reload() {
    plants = await getPlants(true);
  }
  $effect(() => {
    reload();
  });

  function edit(plant: PlantDefinition) {
    editing = plant;
    creating = false;
    name = plant.name;
    category = plant.category;
    minutes = Math.ceil(plant.min_focus_secs / 60);
    accent = plant.accent;
    smallSource = mediumSource = largeSource = null;
    error = '';
  }
  function create() {
    editing = null;
    creating = true;
    name = '';
    category = 'flower';
    minutes = 25;
    accent = '#65a653';
    smallSource = mediumSource = largeSource = null;
    error = '';
  }
  async function pick(stage: 'small' | 'medium' | 'large') {
    const path = await openPlantImagePicker();
    if (!path) return;
    if (stage === 'small') smallSource = path;
    if (stage === 'medium') mediumSource = path;
    if (stage === 'large') largeSource = path;
  }
  function filename(path: string | null) {
    return path?.split(/[\\/]/).pop() ?? m.plant_choose_image();
  }
  async function submit() {
    if (!name.trim()) {
      error = m.plant_name_required();
      return;
    }
    if (creating && (!smallSource || !mediumSource || !largeSource)) {
      error = m.plant_images_required();
      return;
    }
    saving = true;
    error = '';
    const input: PlantInput = {
      id: editing?.id ?? null,
      name: name.trim(),
      category,
      min_focus_secs: Math.max(60, minutes * 60),
      accent,
      small_icon_source_path: smallSource,
      medium_icon_source_path: mediumSource,
      large_icon_source_path: largeSource,
    };
    try {
      await savePlant(input);
      await reload();
      editing = null;
      creating = false;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
  async function toggleHidden(plant: PlantDefinition) {
    await setPlantHidden(plant.id, !plant.hidden);
    await reload();
  }
  async function restore(plant: PlantDefinition) {
    await restorePlantDefault(plant.id);
    await reload();
    editing = null;
  }
  async function remove(plant: PlantDefinition) {
    saving = true;
    error = '';
    try {
      await deletePlant(plant.id);
      await reload();
      editing = null;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<section class="manager" aria-label="Manage plants">
  <header>
    <div>
      <small>{m.plant_library()}</small>
      <h2>{m.plant_manage()}</h2>
    </div>
    <button onclick={onclose} aria-label="Close">x</button>
  </header>
  {#if editing || creating}
    <div class="form">
      <div class="form-title">
        <button
          onclick={() => {
            editing = null;
            creating = false;
          }}>{m.plant_back()}</button
        ><strong>{creating ? m.plant_new() : m.plant_edit({ name: editing?.name ?? '' })}</strong>
      </div>
      <label>{m.plant_name()}<input bind:value={name} maxlength="40" /></label>
      <div class="pair">
        <label
          >{m.plant_category()}<select bind:value={category}
            ><option value="tree">Tree</option><option value="flower">Flower</option><option
              value="succulent">Succulent</option
            ><option value="groundcover">Groundcover</option></select
          ></label
        >
        <label
          >{m.plant_minimum_minutes()}<input
            type="number"
            min="1"
            max="90"
            bind:value={minutes}
          /></label
        >
      </div>
      <label>{m.plant_accent()}<input type="color" bind:value={accent} /></label>
      <div class="images">
        <button onclick={() => pick('small')}
          ><span>{m.plant_stage_small()}</span><small
            >{filename(smallSource ?? editing?.small_icon_path ?? null)}</small
          ></button
        >
        <button onclick={() => pick('medium')}
          ><span>{m.plant_stage_medium()}</span><small
            >{filename(mediumSource ?? editing?.medium_icon_path ?? null)}</small
          ></button
        >
        <button onclick={() => pick('large')}
          ><span>{m.plant_stage_large()}</span><small
            >{filename(largeSource ?? editing?.large_icon_path ?? null)}</small
          ></button
        >
      </div>
      {#if error}<p class="error">{error}</p>{/if}
      <button class="save" disabled={saving} onclick={submit}
        >{saving ? m.plant_saving() : m.plant_save()}</button
      >
      {#if editing?.is_builtin}<button class="restore" onclick={() => restore(editing!)}
          >{m.plant_restore_defaults()}</button
        >{/if}
      {#if editing && !editing.is_builtin}<button
          class="delete"
          disabled={saving}
          onclick={() => remove(editing!)}>{m.plant_delete_permanently()}</button
        >{/if}
    </div>
  {:else}
    <button class="create" onclick={create}>+ {m.plant_create()}</button>
    <div class="list">
      {#each plants as plant}
        <article class:hidden={plant.hidden}>
          <span class="thumb"
            ><PlantIllustration plantId={plant.id} iconPath={plant.large_icon_path} /></span
          >
          <div>
            <strong>{plant.name}</strong><small
              >{plant.is_builtin ? m.plant_builtin() : m.plant_custom()} · {plant.category}</small
            >
          </div>
          <button disabled={plant.id === activePlantId} onclick={() => edit(plant)}
            >{m.plant_edit({ name: '' }).trim()}</button
          >
          <button disabled={plant.id === activePlantId} onclick={() => toggleHidden(plant)}
            >{plant.hidden ? m.plant_restore() : m.plant_hide()}</button
          >
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .manager {
    position: absolute;
    z-index: 12;
    inset: 9px;
    padding: 14px;
    overflow: hidden;
    display: flex;
    min-height: 0;
    flex-direction: column;
    border-radius: 20px;
    color: var(--color-foreground);
    background: color-mix(in oklch, var(--color-background-light) 96%, #000);
    box-shadow: 0 18px 45px color-mix(in oklch, #000 35%, transparent);
  }
  header,
  .form-title,
  article {
    display: flex;
    align-items: center;
  }
  header {
    justify-content: space-between;
    margin-bottom: 10px;
  }
  header small {
    color: var(--color-foreground-darker);
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }
  h2 {
    font-size: 17px;
  }
  header button {
    width: 27px;
    height: 27px;
    border: 0;
    border-radius: 50%;
    color: inherit;
    background: var(--color-hover);
    cursor: pointer;
  }
  .create,
  .save {
    width: 100%;
    border: 0;
    border-radius: 10px;
    padding: 9px;
    color: var(--color-background);
    background: var(--color-accent);
    font-weight: 700;
    cursor: pointer;
  }
  .list {
    min-height: 0;
    flex: 1;
    margin-top: 9px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  article {
    min-height: 58px;
    gap: 7px;
    padding: 5px 7px;
    border: 1px solid var(--color-separator);
    border-radius: 11px;
    background: color-mix(in oklch, var(--color-background) 45%, transparent);
  }
  article.hidden {
    opacity: 0.55;
  }
  article .thumb {
    width: 45px;
    height: 45px;
    flex: none;
  }
  article div {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  article strong {
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  article small {
    font-size: 8px;
    color: var(--color-foreground-darker);
  }
  article button,
  .form-title button,
  .restore {
    border: 0;
    border-radius: 7px;
    padding: 5px 6px;
    color: inherit;
    background: var(--color-hover);
    font-size: 9px;
    cursor: pointer;
  }
  .delete {
    width: 100%;
    border: 1px solid var(--color-focus-round);
    border-radius: 8px;
    padding: 7px;
    color: var(--color-focus-round);
    background: transparent;
    cursor: pointer;
  }
  article button:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .form-title {
    gap: 8px;
  }
  .form label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    color: var(--color-foreground-darker);
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }
  input,
  select {
    min-width: 0;
    border: 1px solid var(--color-separator);
    border-radius: 8px;
    padding: 7px;
    color: var(--color-foreground);
    background: var(--color-background);
    font: 11px 'Mona Sans';
  }
  input[type='color'] {
    width: 100%;
    height: 32px;
    padding: 3px;
  }
  .pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .images {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 5px;
  }
  .images button {
    min-width: 0;
    height: 51px;
    border: 1px dashed var(--color-separator);
    border-radius: 8px;
    color: inherit;
    background: transparent;
    display: flex;
    flex-direction: column;
    justify-content: center;
    cursor: pointer;
  }
  .images small {
    max-width: 75px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-foreground-darker);
    font-size: 7px;
  }
  .error {
    color: var(--color-focus-round);
    font-size: 9px;
  }
  .restore {
    width: 100%;
  }
</style>
