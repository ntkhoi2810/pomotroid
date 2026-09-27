<script lang="ts">
  import type { MotionActivity, WeatherType } from '$lib/types';

  interface Props {
    weather: WeatherType;
    activity?: MotionActivity;
    compact?: boolean;
  }

  let { weather, activity = 'idle', compact = false }: Props = $props();

  const rainDrops = Array.from({ length: 20 }, (_, index) => ({
    left: (index * 37 + 7) % 100,
    delay: -((index * 0.17) % 1.1),
    duration: 0.62 + (index % 5) * 0.08,
  }));
  const leaves = Array.from({ length: 8 }, (_, index) => ({
    top: 34 + ((index * 23) % 48),
    delay: -((index * 0.61) % 4.2),
    duration: 3.2 + (index % 4) * 0.45,
  }));
</script>

<div
  class="weather-scene"
  class:compact
  class:paused={activity === 'paused'}
  class:idle={activity === 'idle'}
  data-weather={weather}
  aria-hidden="true"
>
  <div class="sky-wash"></div>
  <div class="sun"><span></span></div>

  <div class="cloud cloud-one"><i></i><i></i></div>
  <div class="cloud cloud-two"><i></i><i></i></div>
  <div class="cloud cloud-three"><i></i><i></i></div>

  <div class="wind-lines">
    <span></span><span></span><span></span>
  </div>

  <div class="rain-field">
    {#each rainDrops as drop, index}
      <i
        class:hidden-drop={compact && index > 10}
        style="--left: {drop.left}%; --delay: {drop.delay}s; --fall-duration: {drop.duration}s"
      ></i>
    {/each}
  </div>

  <div class="leaves">
    {#each leaves as leaf, index}
      <i
        class:hidden-leaf={compact && index > 3}
        style="--top: {leaf.top}%; --delay: {leaf.delay}s; --leaf-duration: {leaf.duration}s"
      ></i>
    {/each}
  </div>
</div>

<style>
  .weather-scene,
  .sky-wash,
  .rain-field,
  .leaves {
    position: absolute;
    inset: 0;
  }
  .weather-scene {
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
    transition: background 0.8s ease;
  }
  .sky-wash {
    background: linear-gradient(155deg, transparent 25%, var(--weather-tint, transparent));
    transition: background 0.8s ease;
  }
  .weather-scene[data-weather='sunny'] {
    --weather-tint: color-mix(in oklch, #ffe9a6 18%, transparent);
    --cloud-color: color-mix(in oklch, #fff 48%, var(--color-foreground) 8%);
  }
  .weather-scene[data-weather='rain'] {
    --weather-tint: color-mix(in oklch, #58758d 24%, transparent);
    --cloud-color: color-mix(in oklch, var(--color-foreground) 28%, #667789);
  }
  .weather-scene[data-weather='wind'] {
    --weather-tint: color-mix(in oklch, #d8f1e5 12%, transparent);
    --cloud-color: color-mix(in oklch, #fff 34%, var(--color-foreground) 12%);
  }
  .weather-scene[data-weather='storm'] {
    --weather-tint: color-mix(in oklch, #182a46 46%, transparent);
    --cloud-color: color-mix(in oklch, var(--color-foreground) 18%, #38485e);
  }
  .sun {
    position: absolute;
    width: 112px;
    height: 112px;
    right: -30px;
    top: -38px;
    border-radius: 50%;
    opacity: 0;
    background: color-mix(in oklch, #fff3ad 42%, transparent);
    box-shadow: 0 0 38px color-mix(in oklch, #ffe28b 35%, transparent);
    transition: opacity 0.8s ease;
  }
  .sun span {
    position: absolute;
    inset: 27px;
    border-radius: inherit;
    background: color-mix(in oklch, #fff2a6 78%, var(--color-background-light));
  }
  [data-weather='sunny'] .sun {
    opacity: 1;
    animation: sun-breathe 6s ease-in-out infinite;
  }
  .cloud {
    position: absolute;
    left: -70px;
    width: 58px;
    height: 13px;
    border-radius: 999px;
    opacity: 0.72;
    background: var(--cloud-color);
    box-shadow: 0 5px 12px color-mix(in oklch, #000 8%, transparent);
    animation: cloud-drift var(--cloud-speed) linear infinite;
  }
  .cloud i {
    position: absolute;
    bottom: 0;
    border-radius: 50%;
    background: inherit;
  }
  .cloud i:first-child {
    width: 25px;
    height: 25px;
    left: 10px;
  }
  .cloud i:last-child {
    width: 19px;
    height: 19px;
    left: 30px;
  }
  .cloud-one {
    --cloud-speed: 24s;
    top: 68px;
    animation-delay: -8s;
  }
  .cloud-two {
    --cloud-speed: 31s;
    top: 104px;
    opacity: 0.48;
    scale: 0.72;
    animation-delay: -22s;
  }
  .cloud-three {
    --cloud-speed: 20s;
    top: 42px;
    opacity: 0.36;
    scale: 0.5;
    animation-delay: -15s;
  }
  [data-weather='wind'] .cloud,
  [data-weather='storm'] .cloud {
    --cloud-speed: 9s;
  }
  [data-weather='storm'] .cloud {
    opacity: 0.9;
  }
  .wind-lines {
    position: absolute;
    inset: 85px 0 auto;
    height: 100px;
    opacity: 0;
  }
  .wind-lines span {
    position: absolute;
    left: -80px;
    width: 74px;
    height: 1px;
    border-radius: 50%;
    background: color-mix(in oklch, var(--color-foreground) 34%, transparent);
    animation: wind-pass 2.8s ease-in-out infinite;
  }
  .wind-lines span:nth-child(2) {
    top: 32px;
    width: 105px;
    animation-delay: -1.5s;
  }
  .wind-lines span:nth-child(3) {
    top: 68px;
    width: 54px;
    animation-delay: -0.7s;
  }
  [data-weather='wind'] .wind-lines,
  [data-weather='storm'] .wind-lines {
    opacity: 1;
  }
  .rain-field {
    z-index: 1;
    opacity: 0;
    transform: skewX(-9deg);
  }
  .rain-field i {
    position: absolute;
    top: -24px;
    left: var(--left);
    width: 1px;
    height: 16px;
    border-radius: 999px;
    background: color-mix(in oklch, #cfe9ff 70%, var(--color-foreground));
    animation: rain-fall var(--fall-duration) linear infinite;
    animation-delay: var(--delay);
  }
  [data-weather='rain'] .rain-field {
    opacity: 0.52;
  }
  [data-weather='storm'] .rain-field {
    opacity: 0.82;
  }
  [data-weather='storm'] .rain-field i {
    height: 23px;
  }
  .leaves {
    opacity: 0;
  }
  .leaves i {
    position: absolute;
    top: var(--top);
    left: -18px;
    width: 8px;
    height: 5px;
    border-radius: 80% 15% 80% 15%;
    background: color-mix(in oklch, var(--color-short-round) 72%, #a8b85e);
    animation: leaf-flight var(--leaf-duration) linear infinite;
    animation-delay: var(--delay);
  }
  [data-weather='wind'] .leaves,
  [data-weather='storm'] .leaves {
    opacity: 0.7;
  }
  [data-weather='storm'] .leaves {
    opacity: 0.48;
  }
  .paused .cloud,
  .idle .cloud {
    animation-duration: 46s;
  }
  .paused .wind-lines span,
  .idle .wind-lines span {
    animation-duration: 7s;
  }
  .paused .rain-field i,
  .idle .rain-field i {
    animation-duration: 1.8s;
  }
  .paused .leaves i,
  .idle .leaves i {
    animation-duration: 8s;
  }
  .compact .cloud-three,
  .hidden-drop,
  .hidden-leaf {
    display: none;
  }
  @keyframes cloud-drift {
    to {
      transform: translate3d(410px, 0, 0);
    }
  }
  @keyframes sun-breathe {
    50% {
      opacity: 0.75;
      transform: scale(1.05);
    }
  }
  @keyframes rain-fall {
    to {
      transform: translate3d(-28px, 340px, 0);
    }
  }
  @keyframes wind-pass {
    45%,
    100% {
      transform: translate3d(410px, -8px, 0) scaleX(1.4);
      opacity: 0;
    }
    12% {
      opacity: 0.7;
    }
  }
  @keyframes leaf-flight {
    25% {
      transform: translate3d(105px, -16px, 0) rotate(140deg);
    }
    55% {
      transform: translate3d(230px, 13px, 0) rotate(310deg);
    }
    to {
      transform: translate3d(365px, -10px, 0) rotate(520deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .sun,
    .cloud,
    .wind-lines span,
    .rain-field i,
    .leaves i {
      animation: none !important;
    }
    .cloud-one {
      left: 24px;
    }
    .cloud-two {
      left: 225px;
    }
    .cloud-three {
      left: 145px;
    }
    .rain-field i,
    .leaves,
    .wind-lines {
      display: none;
    }
  }
</style>
