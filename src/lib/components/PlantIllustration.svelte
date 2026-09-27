<script lang="ts">
  import type { GrowthStage, MotionActivity, WeatherType } from '$lib/types';
  import { convertFileSrc } from '@tauri-apps/api/core';

  interface Props {
    plantId?: string | null;
    iconPath?: string | null;
    progress?: number;
    stage?: GrowthStage;
    label?: string;
    category?: string;
    weather?: WeatherType | null;
    activity?: MotionActivity;
  }

  let {
    plantId = null,
    iconPath = null,
    progress = 1,
    stage,
    label = '',
    category = 'tree',
    weather = null,
    activity = 'idle',
  }: Props = $props();
  const stageScale = $derived(stage === 'small' ? 0.62 : stage === 'medium' ? 0.82 : 1);
  const scale = $derived(stage ? stageScale : 0.18 + Math.max(0, Math.min(1, progress)) * 0.82);
</script>

{#if iconPath}
  <div
    class="plant custom"
    role={label ? 'img' : 'presentation'}
    aria-label={label || undefined}
    data-weather={weather ?? undefined}
    data-activity={activity}
    data-category={category}
  >
    <span class="custom-shadow"></span>
    <span class="weather-motion">
      <img src={convertFileSrc(iconPath)} alt="" style="--growth: {scale}" />
    </span>
  </div>
{:else if plantId}
  <svg
    class="plant"
    viewBox="0 0 180 180"
    role={label ? 'img' : 'presentation'}
    aria-label={label || undefined}
    style="--growth: {scale}"
    data-weather={weather ?? undefined}
    data-activity={activity}
    data-category={category}
  >
    <ellipse class="shadow" cx="90" cy="162" rx="42" ry="8" />
    <g class="weather-motion">
      <g class="growing">
        {#if plantId === 'clover'}
          <path class="stem" d="M90 160 C88 139 89 121 88 102" />
          <path
            class="stem thin"
            d="M89 130 C72 125 65 114 58 104 M89 124 C105 118 112 107 117 97"
          />
          <g class="leaf clover-leaf">
            <circle cx="51" cy="98" r="13" /><circle cx="66" cy="94" r="13" />
            <circle cx="59" cy="111" r="13" />
          </g>
          <g class="leaf clover-leaf">
            <circle cx="111" cy="91" r="12" /><circle cx="125" cy="88" r="12" />
            <circle cx="119" cy="103" r="12" />
          </g>
          <g class="leaf clover-leaf">
            <circle cx="80" cy="92" r="14" /><circle cx="96" cy="90" r="14" />
            <circle cx="89" cy="106" r="14" />
          </g>
        {:else if plantId === 'daisy'}
          <path class="stem" d="M90 160 C85 132 92 105 90 78" />
          <path class="leaf" d="M88 130 C65 123 62 109 65 103 C82 104 90 114 88 130Z" />
          <path class="leaf" d="M91 116 C109 109 119 114 121 120 C109 131 99 129 91 116Z" />
          <g class="flower daisy">
            {#each [0, 45, 90, 135, 180, 225, 270, 315] as angle}
              <ellipse cx="90" cy="59" rx="8" ry="19" transform="rotate({angle} 90 78)" />
            {/each}
            <circle class="flower-center" cx="90" cy="78" r="12" />
          </g>
        {:else if plantId === 'cactus'}
          <path
            class="cactus"
            d="M75 159 L75 76 C75 61 86 52 97 56 C105 59 108 68 106 82 L104 99 L116 91 L117 77 C118 69 124 65 131 68 C137 71 138 77 136 84 L132 105 C130 114 119 121 104 124 L103 159Z"
          />
          <path
            class="cactus-line"
            d="M87 62 L87 153 M98 63 L96 151 M80 83 L101 83 M120 77 L129 78"
          />
          <g class="flower cactus-flower"
            ><circle cx="93" cy="52" r="9" /><circle cx="83" cy="55" r="8" /><circle
              cx="102"
              cy="57"
              r="8"
            /><circle class="flower-center" cx="93" cy="58" r="6" /></g
          >
        {:else if plantId === 'lavender'}
          {#each [66, 78, 90, 102, 114] as x, i}
            <path
              class="stem thin"
              d="M90 160 C{x - 90} 142 {x} 105 {x} {67 + Math.abs(i - 2) * 7}"
            />
            {#each [0, 1, 2, 3, 4] as n}
              <circle
                class="lavender"
                cx={x + (n % 2 ? 4 : -3)}
                cy={75 + n * 8 + Math.abs(i - 2) * 5}
                r="6"
              />
            {/each}
          {/each}
          <path
            class="leaf"
            d="M89 139 C67 129 64 119 67 114 C79 116 87 124 89 139Z M92 143 C109 130 120 130 124 135 C116 147 104 150 92 143Z"
          />
        {:else}
          <path
            class="trunk"
            class:oak={plantId === 'oak'}
            d={plantId === 'oak'
              ? 'M73 161 C79 137 80 118 78 98 L61 79 L67 72 L85 88 L91 55 L99 57 L97 91 L118 73 L124 80 L101 105 C100 126 104 145 111 161Z'
              : 'M82 161 C86 137 87 116 88 95 L74 81 L80 76 L91 86 L96 63 L103 66 L99 99 C99 121 102 142 108 161Z'}
          />
          {#if plantId === 'pine'}
            <path class="pine back" d="M96 34 L53 101 L72 99 L45 137 L145 137 L119 99 L138 101Z" />
            <path class="pine front" d="M96 48 L67 101 L82 99 L61 127 L131 127 L111 99 L126 101Z" />
          {:else if plantId === 'cherry'}
            <g class="canopy cherry">
              <circle cx="65" cy="78" r="29" /><circle cx="91" cy="57" r="33" />
              <circle cx="119" cy="79" r="31" /><circle cx="92" cy="91" r="35" />
              <circle class="blossom" cx="58" cy="72" r="5" /><circle
                class="blossom"
                cx="83"
                cy="48"
                r="5"
              />
              <circle class="blossom" cx="111" cy="63" r="5" /><circle
                class="blossom"
                cx="99"
                cy="88"
                r="5"
              />
            </g>
          {:else if plantId === 'maple'}
            <g class="canopy maple">
              <circle cx="62" cy="81" r="29" /><circle cx="86" cy="56" r="33" />
              <circle cx="119" cy="77" r="31" /><circle cx="94" cy="92" r="36" />
            </g>
          {:else}
            <g class="canopy oak-canopy">
              <circle cx="51" cy="88" r="28" /><circle cx="66" cy="60" r="33" />
              <circle cx="98" cy="48" r="38" /><circle cx="128" cy="69" r="34" />
              <circle cx="132" cy="99" r="29" /><circle cx="91" cy="91" r="45" />
            </g>
          {/if}
        {/if}
      </g>
    </g>
  </svg>
{/if}

<style>
  .plant {
    width: 100%;
    height: 100%;
    overflow: visible;
  }
  .custom {
    position: relative;
    display: grid;
    place-items: end center;
  }
  .custom .weather-motion {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: end center;
    transform-box: border-box;
  }
  .custom img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    transform-origin: center bottom;
    transform: scale(var(--growth));
    transition: transform 0.8s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .custom-shadow {
    position: absolute;
    left: 27%;
    right: 27%;
    bottom: 5%;
    height: 9%;
    border-radius: 50%;
    background: color-mix(in oklch, var(--color-foreground) 13%, transparent);
  }
  .growing {
    transform-box: fill-box;
    transform-origin: center bottom;
    transform: scale(var(--growth));
    transition: transform 0.8s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .weather-motion {
    transform-box: fill-box;
    transform-origin: center bottom;
  }
  .shadow {
    fill: color-mix(in oklch, var(--color-foreground) 13%, transparent);
  }
  .stem,
  .trunk {
    fill: none;
    stroke: #6d5138;
    stroke-width: 8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .stem.thin {
    stroke-width: 4;
  }
  .leaf,
  .clover-leaf {
    fill: #65a653;
    stroke: #477d3d;
    stroke-width: 2;
    stroke-linejoin: round;
  }
  .flower {
    fill: #f4efe1;
  }
  .flower-center {
    fill: #e6b94e;
  }
  .cactus {
    fill: #559c68;
    stroke: #39764e;
    stroke-width: 3;
  }
  .cactus-line {
    fill: none;
    stroke: #82bc79;
    stroke-width: 3;
    stroke-linecap: round;
  }
  .cactus-flower {
    fill: #e57d8e;
  }
  .lavender {
    fill: #9270b7;
  }
  .trunk {
    fill: #765238;
    stroke: #5b3e2c;
    stroke-width: 3;
  }
  .trunk.oak {
    fill: #6a4b33;
  }
  .pine {
    stroke-linejoin: round;
  }
  .pine.back {
    fill: #35664f;
  }
  .pine.front {
    fill: #4c8665;
  }
  .canopy {
    fill: #6aa257;
  }
  .cherry {
    fill: #dfa0ae;
  }
  .cherry .blossom {
    fill: #f7d5dc;
  }
  .maple {
    fill: #cf7042;
  }
  .oak-canopy {
    fill: #668b4f;
  }
  @media (prefers-reduced-motion: reduce) {
    .weather-motion {
      animation: none !important;
      transform: none !important;
    }
    .growing,
    .custom img {
      transition: none;
    }
  }
</style>
