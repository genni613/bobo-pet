<script lang="ts">
  import { onMount } from 'svelte';
  import spriteSheet from './assets/orange-pet/spritesheet.webp';
  import { defaultConfig, type PetConfig, type TimerPhase } from './lib/types';
  import { loadBootstrap, saveConfig, setPanelVisible } from './lib/desktop';
  import { getCurrentWindow, primaryMonitor, LogicalPosition, LogicalSize } from '@tauri-apps/api/window';

  const isPetView = typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('view') === 'pet';
  const appWindow = getCurrentWindow();

  const PET_WINDOW_WIDTH = 144;
  const PET_WINDOW_HEIGHT = 156;
  const PET_HALF_X = PET_WINDOW_WIDTH / 2;
  const PET_HALF_Y = PET_WINDOW_HEIGHT / 2;
  const FLOOR_PADDING = 32;
  const SIDE_PADDING = 18;
  const MAX_DRAG_SPEED = 1800;
  const CEILING_PADDING = 8;
  const GRAVITY = 1900;
  const AIR_DRAG = 0.992;
  const SPRITE_COLUMNS = 8;
  const SPRITE_ROWS = 9;
  const SPRITE_CELL_WIDTH = 192;
  const SPRITE_CELL_HEIGHT = 208;
  const SPRITE_MAIN_WIDTH = 124;
  const SPRITE_MINI_WIDTH = 60;

  type SpriteState =
    | 'idle'
    | 'running-right'
    | 'running-left'
    | 'waving'
    | 'jumping'
    | 'failed'
    | 'waiting'
    | 'running'
    | 'review';

  const SPRITE_VARIANTS: Record<SpriteState, { row: number; frames: number; speedMs: number }> = {
    idle: { row: 0, frames: 6, speedMs: 180 },
    'running-right': { row: 1, frames: 8, speedMs: 90 },
    'running-left': { row: 2, frames: 8, speedMs: 90 },
    waving: { row: 3, frames: 4, speedMs: 140 },
    jumping: { row: 4, frames: 5, speedMs: 120 },
    failed: { row: 5, frames: 8, speedMs: 110 },
    waiting: { row: 6, frames: 6, speedMs: 180 },
    running: { row: 7, frames: 6, speedMs: 95 },
    review: { row: 8, frames: 6, speedMs: 140 }
  };

  let viewportWidth = isPetView ? PET_WINDOW_WIDTH : 420;
  let viewportHeight = isPetView ? PET_WINDOW_HEIGHT : 820;
  let config: PetConfig = { ...defaultConfig };
  let booted = false;

  let panelOpen = !isPetView;
  let phase: TimerPhase = 'idle';
  let timerEndsAt: number | null = null;
  let remainingMs = defaultConfig.remainingMs;
  let activeFocusLengthMs = defaultConfig.activeFocusLengthMs;
  let focusSessions = 0;
  let mood = 'idle';
  let moodLine = '轻点我，我会陪你专注。';
  let moodStickyUntil = 0;
  let now = Date.now();
  let idleWave = 0;
  let waterDue = false;
  let standDue = false;
  let focusProgress = 0;
  let timerLabel = '25:00';
  let focusAccumulatedMinutes = 0;
  let hydrationScore = 100;
  let standingScore = 100;
  let petVisualY = 0;
  let phaseLabel = '待命';

  let petX = 300;
  let petY = 0;
  let petVx = 0;
  let petVy = 0;
  let petScaleX = 1;
  let petScaleY = 1;
  let petRotation = 0;
  let isDragging = false;
  let isAirborne = false;
  let dragDistance = 0;
  let dragStartPetX = 0;
  let dragStartPetY = 0;
  let dragStartScreenX = 0;
  let dragStartScreenY = 0;
  let lastPointerScreenX = 0;
  let lastPointerScreenY = 0;
  let lastPointerAt = 0;
  let suppressClick = false;

  let raf = 0;
  let interval = 0;
  let frameAt = 0;
  let spriteState: SpriteState = 'idle';
  let spriteSpec = SPRITE_VARIANTS.idle;

  function idleBlinkFrame(t: number): number {
    const cycle = 3800;
    const blinkStart = 3000;
    const p = t % cycle;

    if (p < blinkStart) {
      return [0, 1, 5, 0, 1][Math.floor(p / 600) % 5];
    }

    const bp = p - blinkStart;
    return [2, 3, 4, 3][Math.min(Math.floor(bp / 200), 3)];
  }
  let spriteFrame = 0;
  let lastPersistSignature = '';

  type IdleAction = 'none' | 'wander-left' | 'wander-right' | 'wave' | 'hop';
  let idleAction: IdleAction = 'none';
  let idleActionEndsAt = 0;
  let nextIdleActionAt = Date.now() + 3000;

  const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
  const lerp = (from: number, to: number, amount: number) => from + (to - from) * amount;
  const floorY = () => viewportHeight - PET_HALF_Y - FLOOR_PADDING;

  const spriteVars = (width: number) => {
    const height = (width * SPRITE_CELL_HEIGHT) / SPRITE_CELL_WIDTH;
    return [
      `--sprite-sheet:url(${spriteSheet})`,
      `--sprite-columns:${SPRITE_COLUMNS}`,
      `--sprite-rows:${SPRITE_ROWS}`,
      `--sprite-display-cell-width:${width}px`,
      `--sprite-display-cell-height:${height}px`,
      `--sprite-row:${spriteSpec.row}`,
      `--sprite-frame:${spriteFrame}`,
      `width:${width}px`,
      `height:${height}px`
    ].join('; ');
  };

  const formatClock = (ms: number) => {
    const totalSeconds = Math.max(0, Math.ceil(ms / 1000));
    const minutes = Math.floor(totalSeconds / 60)
      .toString()
      .padStart(2, '0');
    const seconds = (totalSeconds % 60).toString().padStart(2, '0');
    return `${minutes}:${seconds}`;
  };

  const formatMinutes = (minutes: number) => {
    if (minutes < 60) {
      return `${minutes} 分钟`;
    }
    const hours = Math.floor(minutes / 60);
    const rest = minutes % 60;
    return rest === 0 ? `${hours} 小时` : `${hours} 小时 ${rest} 分钟`;
  };

  const snapshotConfig = (): PetConfig => ({
    ...config,
    focusMinutes: config.focusMinutes,
    breakMinutes: config.breakMinutes,
    waterIntervalMinutes: config.waterIntervalMinutes,
    standIntervalMinutes: config.standIntervalMinutes,
    petX,
    petY: Math.min(petY, floorY()),
    panelOpen,
    phase,
    remainingMs,
    activeFocusLengthMs,
    focusSessions,
    lastWaterAt: config.lastWaterAt,
    lastStandAt: config.lastStandAt
  });

  const persistConfig = async () => {
    if (!booted) {
      return;
    }
    config = snapshotConfig();
    await saveConfig(config);
  };

  const runtimeSignature = () =>
    [
      phase,
      Math.ceil(remainingMs / 1000),
      Math.ceil(activeFocusLengthMs / 1000),
      focusSessions,
      config.lastWaterAt,
      config.lastStandAt,
      panelOpen ? 1 : 0
    ].join('|');

  const maybePersistRuntime = () => {
    const signature = runtimeSignature();
    if (signature === lastPersistSignature) {
      return;
    }
    lastPersistSignature = signature;
    void persistConfig();
  };

  const setMood = (nextMood: string, line: string, stickyMs = 0) => {
    mood = nextMood;
    moodLine = line;
    moodStickyUntil = stickyMs > 0 ? Date.now() + stickyMs : 0;
  };

  const resolveSpriteState = (): SpriteState => {
    if (isDragging) {
      if (Math.abs(petVx) > 140) {
        return petVx >= 0 ? 'running-right' : 'running-left';
      }
      return 'running';
    }
    if (isAirborne) {
      return 'jumping';
    }
    if (mood === 'celebrate') {
      return 'waving';
    }
    if (mood === 'landing') {
      return 'jumping';
    }
    if (mood === 'hydrated' || mood === 'recovered') {
      return 'review';
    }
    if (mood === 'paused') {
      return 'idle';
    }
    if (mood === 'focus') {
      return 'waiting';
    }
    if (mood === 'break') {
      return 'review';
    }
    if (mood === 'dragging') {
      return 'running';
    }
    if (phase === 'focus') {
      return 'waiting';
    }
    if (phase === 'break') {
      return 'review';
    }
    if (phase === 'paused-focus' || phase === 'paused-break') {
      return 'idle';
    }
    if (waterDue || standDue) {
      return 'review';
    }
    if (idleAction === 'wander-left') return 'running-left';
    if (idleAction === 'wander-right') return 'running-right';
    if (idleAction === 'wave') return 'waving';
    return 'idle';
  };

  const refreshMood = () => {
    if (moodStickyUntil > Date.now()) {
      return;
    }
    if (isDragging) {
      setMood('dragging', '慢一点拎，我会自己弹回来。');
      return;
    }
    if (waterDue) {
      setMood('water', '该喝水了，我先替你着急。');
      return;
    }
    if (standDue) {
      setMood('stand', '起来活动两分钟，回来我还在。');
      return;
    }
    if (phase === 'focus') {
      setMood('focus', '专注进行中，我在帮你蓄力。');
      return;
    }
    if (phase === 'break') {
      setMood('break', '休息一会，我帮你看着时间。');
      return;
    }
    setMood('idle', focusSessions > 0 ? `今天已经完成 ${focusSessions} 次专注。` : '轻点我，我会陪你专注。');
  };

  const updateViewport = async () => {
    if (isPetView) {
      const monitor = await primaryMonitor();
      if (monitor) {
        const scaleFactor = monitor.scaleFactor || 1;
        viewportWidth = monitor.workArea.size.width / scaleFactor;
        viewportHeight = monitor.workArea.size.height / scaleFactor;
      } else {
        viewportWidth = window.innerWidth;
        viewportHeight = window.innerHeight;
      }
    } else {
      viewportWidth = window.innerWidth;
      viewportHeight = window.innerHeight;
    }

    if (!booted) {
      return;
    }

    petX = clamp(petX, PET_HALF_X + SIDE_PADDING, viewportWidth - PET_HALF_X - SIDE_PADDING);
    petY = clamp(petY, PET_HALF_Y + 24, floorY());
    if (isPetView) {
      void syncPetWindow();
    }
  };

  const syncPetWindow = async () => {
    if (!isPetView || !booted) {
      return;
    }
    const left = petX - PET_HALF_X;
    const top = petY - PET_HALF_Y;
    await appWindow.setPosition(new LogicalPosition(left, top));
  };

  const loadState = async () => {
    const bootstrap = await loadBootstrap();
    config = bootstrap.config;
    phase = bootstrap.config.phase ?? 'idle';
    remainingMs = Math.max(0, bootstrap.config.remainingMs ?? bootstrap.config.focusMinutes * 60_000);
    activeFocusLengthMs = Math.max(remainingMs, bootstrap.config.activeFocusLengthMs ?? remainingMs);
    focusSessions = bootstrap.config.focusSessions ?? 0;
    panelOpen = isPetView ? false : true;

    petX = clamp(
      bootstrap.config.petX || viewportWidth / 2,
      PET_HALF_X + SIDE_PADDING,
      viewportWidth - PET_HALF_X - SIDE_PADDING
    );
    petY = clamp(bootstrap.config.petY || floorY(), PET_HALF_Y + 24, floorY());
    timerEndsAt = phase === 'focus' || phase === 'break' ? Date.now() + remainingMs : null;
    now = Date.now();
    booted = true;
    lastPersistSignature = '';
    refreshMood();

    if (isPetView) {
      await appWindow.setSize(new LogicalSize(PET_WINDOW_WIDTH, PET_WINDOW_HEIGHT));
      void syncPetWindow();
    }
  };

  const startFocus = async () => {
    phase = 'focus';
    activeFocusLengthMs = config.focusMinutes * 60_000;
    remainingMs = activeFocusLengthMs;
    timerEndsAt = Date.now() + remainingMs;
    refreshMood();
    await persistConfig();
  };

  const togglePause = async () => {
    if (phase === 'focus' || phase === 'break') {
      remainingMs = Math.max(0, (timerEndsAt ?? Date.now()) - Date.now());
      phase = phase === 'focus' ? 'paused-focus' : 'paused-break';
      timerEndsAt = null;
      setMood('paused', '我替你把时间按住了。', 2400);
      await persistConfig();
      return;
    }
    if (phase === 'paused-focus' || phase === 'paused-break') {
      timerEndsAt = Date.now() + remainingMs;
      phase = phase === 'paused-focus' ? 'focus' : 'break';
      refreshMood();
      await persistConfig();
    }
  };

  const resetTimer = async () => {
    phase = 'idle';
    timerEndsAt = null;
    remainingMs = config.focusMinutes * 60_000;
    activeFocusLengthMs = remainingMs;
    refreshMood();
    await persistConfig();
  };

  const confirmWater = async () => {
    config.lastWaterAt = Date.now();
    setMood('hydrated', '补水完成，我看起来更亮了。', 3200);
    await persistConfig();
  };

  const confirmStand = async () => {
    config.lastStandAt = Date.now();
    setMood('recovered', '活动完成，继续稳住节奏。', 3200);
    await persistConfig();
  };

  const adjustFocusMinutes = async (delta: number) => {
    config.focusMinutes = clamp(config.focusMinutes + delta, 10, 60);
    if (phase === 'idle') {
      remainingMs = config.focusMinutes * 60_000;
      activeFocusLengthMs = remainingMs;
    }
    await persistConfig();
  };

  const adjustBreakMinutes = async (delta: number) => {
    config.breakMinutes = clamp(config.breakMinutes + delta, 3, 20);
    await persistConfig();
  };

  const adjustWaterMinutes = async (delta: number) => {
    config.waterIntervalMinutes = clamp(config.waterIntervalMinutes + delta, 20, 120);
    await persistConfig();
  };

  const adjustStandMinutes = async (delta: number) => {
    config.standIntervalMinutes = clamp(config.standIntervalMinutes + delta, 30, 150);
    await persistConfig();
  };

  const pointerDown = (event: PointerEvent) => {
    if (!isPetView) {
      return;
    }
    event.preventDefault();
    isDragging = true;
    isAirborne = false;
    idleAction = 'none';
    dragDistance = 0;
    suppressClick = false;
    dragStartPetX = petX;
    dragStartPetY = petY;
    dragStartScreenX = event.screenX;
    dragStartScreenY = event.screenY;
    lastPointerScreenX = event.screenX;
    lastPointerScreenY = event.screenY;
    lastPointerAt = performance.now();
    petVx = 0;
    petVy = 0;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    refreshMood();
  };

  const pointerMove = (event: PointerEvent) => {
    if (!isDragging || !isPetView) {
      return;
    }
    const nowAt = performance.now();
    const dt = Math.max(0.008, (nowAt - lastPointerAt) / 1000);
    const dx = event.screenX - lastPointerScreenX;
    const dy = event.screenY - lastPointerScreenY;
    lastPointerScreenX = event.screenX;
    lastPointerScreenY = event.screenY;
    dragDistance += Math.abs(dx) + Math.abs(dy);

    petX = clamp(petX + dx, PET_HALF_X + SIDE_PADDING, viewportWidth - PET_HALF_X - SIDE_PADDING);
    petY = clamp(petY + dy, PET_HALF_Y + 18, floorY());
    petVx = clamp(dx / dt, -MAX_DRAG_SPEED, MAX_DRAG_SPEED);
    petVy = clamp(dy / dt, -MAX_DRAG_SPEED, MAX_DRAG_SPEED);
    petRotation = clamp(petVx * 0.015, -18, 18);
    petScaleX = clamp(1 + Math.abs(petVx) / 2600 + Math.max(-petVy, 0) / 3200, 1, 1.18);
    petScaleY = clamp(1 - Math.abs(petVx) / 3800, 0.84, 1.02);
    lastPointerAt = nowAt;
    void syncPetWindow();
  };

  const pointerUp = async () => {
    if (!isDragging || !isPetView) {
      return;
    }
    isDragging = false;
    suppressClick = dragDistance > 12;

    if (petY < floorY() - 120 || petVy < -220) {
      isAirborne = true;
      petVy = Math.min(petVy, -160);
    } else {
      petVy = -Math.abs(petVy) * 0.16;
      isAirborne = true;
    }

    refreshMood();
    await persistConfig();
  };

  const openPanel = async () => {
    if (!isPetView) {
      return;
    }
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    if (panelOpen) {
      panelOpen = false;
      await persistConfig();
      await setPanelVisible(false);
      return;
    }
    panelOpen = true;
    await persistConfig();
    await setPanelVisible(true);
  };

  const closePanel = async () => {
    if (isPetView) {
      return;
    }
    panelOpen = false;
    await persistConfig();
    await setPanelVisible(false);
  };

  const physicsTick = (dt: number) => {
    if (!isPetView || isDragging) {
      return;
    }

    const floor = floorY();

    if (isAirborne) {
      petVy += GRAVITY * dt;
      petX += petVx * dt;
      petY += petVy * dt;
      petVx *= AIR_DRAG;
      petRotation = clamp(petVx * 0.008, -12, 12);

      const leftBound = PET_HALF_X + SIDE_PADDING;
      const rightBound = viewportWidth - PET_HALF_X - SIDE_PADDING;
      if (petX <= leftBound || petX >= rightBound) {
        petX = clamp(petX, leftBound, rightBound);
        petVx *= -0.35;
      }

      const topBound = PET_HALF_Y + CEILING_PADDING;
      if (petY <= topBound) {
        petY = topBound;
        petVy *= -0.3;
      }

      if (petY >= floor) {
        petY = floor;
        if (Math.abs(petVy) > 110) {
          petVy *= -0.28;
          petVx *= 0.75;
          petScaleX = 1.12;
          petScaleY = 0.86;
          setMood('landing', '落地了，继续保持。');
        } else {
          petVy = 0;
          petVx = 0;
          isAirborne = false;
          refreshMood();
        }
      }
      void syncPetWindow();
    } else {
      petY = floor;
      petRotation = lerp(petRotation, 0, 0.12);
      if (idleAction === 'wander-left' || idleAction === 'wander-right') {
        petX += petVx * dt;
        petX = clamp(petX, PET_HALF_X + SIDE_PADDING, viewportWidth - PET_HALF_X - SIDE_PADDING);
        void syncPetWindow();
      }
    }

    petScaleX = lerp(petScaleX, 1, 0.12);
    petScaleY = lerp(petScaleY, 1, 0.12);
  };

  const timerTick = () => {
    now = Date.now();
    idleWave = now / 1000;

    if (phase === 'focus' || phase === 'break') {
      remainingMs = Math.max(0, (timerEndsAt ?? now) - now);
      if (remainingMs === 0) {
        if (phase === 'focus') {
          focusSessions += 1;
          phase = 'break';
          remainingMs = config.breakMinutes * 60_000;
          timerEndsAt = now + remainingMs;
          config.lastStandAt = now;
          setMood('celebrate', '这一轮完成了，休息一下。', 3800);
        } else {
          phase = 'idle';
          timerEndsAt = null;
          remainingMs = config.focusMinutes * 60_000;
          setMood('ready', '休息结束，下一轮可以开始了。', 2800);
        }
      }
    }

    if (phase === 'idle') {
      remainingMs = config.focusMinutes * 60_000;
    }

    maybePersistRuntime();
    updateIdleBehavior();
  };

  const updateIdleBehavior = () => {
    if (isDragging || isAirborne || phase === 'focus' || phase === 'break') {
      return;
    }

    if (idleAction !== 'none' && now > idleActionEndsAt) {
      if (idleAction === 'wander-left' || idleAction === 'wander-right') {
        petVx = 0;
      }
      idleAction = 'none';
      refreshMood();
    }

    if (idleAction === 'none' && now > nextIdleActionAt) {
      const roll = Math.random();
      if (roll < 0.25) {
        const dir: IdleAction = Math.random() < 0.5 ? 'wander-left' : 'wander-right';
        idleAction = dir;
        petVx = dir === 'wander-left' ? -55 : 55;
        idleActionEndsAt = now + 2000 + Math.random() * 2500;
      } else if (roll < 0.38) {
        idleAction = 'wave';
        idleActionEndsAt = now + 900;
      } else if (roll < 0.48) {
        idleAction = 'hop';
        petVy = -380;
        isAirborne = true;
        idleActionEndsAt = now + 1200;
      }
      nextIdleActionAt = now + 4000 + Math.random() * 7000;
    }
  };

  const tick = (at: number) => {
    if (frameAt === 0) {
      frameAt = at;
    }
    const dt = Math.min(0.032, (at - frameAt) / 1000);
    frameAt = at;
    physicsTick(dt);
    timerTick();
    if (!isDragging && !isAirborne && idleAction === 'none') {
      refreshMood();
    }
  };

  const startLoop = () => {
    if (raf !== 0 || interval !== 0) {
      return;
    }
    frameAt = 0;
    if (isPetView) {
      interval = window.setInterval(() => tick(performance.now()), 33);
      return;
    }
    const step = (at: number) => {
      tick(at);
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
  };

  const stopLoop = () => {
    if (interval !== 0) {
      clearInterval(interval);
      interval = 0;
    }
    if (raf !== 0) {
      cancelAnimationFrame(raf);
      raf = 0;
    }
    frameAt = 0;
  };

  const handleVisibilityChange = async () => {
    if (document.visibilityState === 'hidden') {
      await persistConfig();
      if (!isPetView) {
        stopLoop();
      }
      return;
    }

    await updateViewport();
    await loadState();
    startLoop();
  };

  onMount(() => {
    const boot = async () => {
      try {
        await updateViewport();
        await loadState();
      } catch (e) {
        console.error('[pet] boot failed:', e);
        booted = true;
      }
      startLoop();
    };

    void boot();

    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('resize', updateViewport);

    return () => {
      stopLoop();
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('resize', updateViewport);
    };
  });

  $: waterDue = now - config.lastWaterAt >= config.waterIntervalMinutes * 60_000;
  $: standDue = now - config.lastStandAt >= config.standIntervalMinutes * 60_000;
  $: focusProgress =
    phase === 'focus'
      ? 1 - remainingMs / activeFocusLengthMs
      : phase === 'break'
        ? 1 - remainingMs / (config.breakMinutes * 60_000)
        : 0;
  $: timerLabel = formatClock(remainingMs);
  $: focusAccumulatedMinutes =
    focusSessions * config.focusMinutes +
    (phase === 'focus' ? Math.floor((activeFocusLengthMs - remainingMs) / 60_000) : 0);
  $: hydrationScore = Math.round(
    Math.max(20, 100 - ((now - config.lastWaterAt) / (config.waterIntervalMinutes * 60_000)) * 100)
  );
  $: standingScore = Math.round(
    Math.max(20, 100 - ((now - config.lastStandAt) / (config.standIntervalMinutes * 60_000)) * 100)
  );
  $: petVisualY = petY + (!isDragging && !isAirborne ? Math.sin(idleWave * 1.7) * 6 : 0);
  $: phaseLabel =
    phase === 'focus'
      ? '专注中'
      : phase === 'break'
        ? '休息中'
        : phase === 'paused-focus' || phase === 'paused-break'
          ? '已暂停'
          : '待命';
  $: spriteState = resolveSpriteState();
  $: spriteSpec = SPRITE_VARIANTS[spriteState];
  $: spriteFrame = spriteState === 'idle'
    ? idleBlinkFrame(now)
    : Math.floor(now / spriteSpec.speedMs) % spriteSpec.frames;
</script>

<svelte:window on:pointermove={pointerMove} on:pointerup={pointerUp} on:pointercancel={pointerUp} />

{#if isPetView}
  <div class="shell pet-shell">
    <button
      class={`pet pet-window pet-${mood}`}
      type="button"
      aria-label="桌面宠物"
      on:pointerdown={pointerDown}
      on:click={openPanel}
      style={`--pet-scale-x:${petScaleX}; --pet-scale-y:${petScaleY}; --pet-rotate:${petRotation}deg;`}
    >
      <div class="pet-frame" aria-hidden="true">
        <div class="pet-sprite main-pet-sprite" style={spriteVars(SPRITE_MAIN_WIDTH)}></div>
      </div>
    </button>
  </div>
{:else}
  <div class="shell panel-shell">
    <div class="atmosphere"></div>
    <div class="orb orb-a"></div>
    <div class="orb orb-b"></div>

    <aside class:open={panelOpen} class="panel glass">
      <div class="panel-head">
        <div class="panel-brand">
          <div class="panel-mark" aria-hidden="true">
            <div class="pet-sprite panel-mark-sprite" style={spriteVars(28)}></div>
          </div>
          <div>
            <p class="eyebrow">Element Pet</p>
            <h1>灵橙专注宠物</h1>
            <p class="panel-subtitle">专注 / 补水 / 起身提醒</p>
          </div>
        </div>
        <button class="ghost panel-close" type="button" on:click={closePanel}>收起</button>
      </div>

      <section class="hero-card glass summary-card">
        <div class="hero-copy">
          <p class="status-chip">{phaseLabel}</p>
          <h2>{moodLine}</h2>
          <p>今天已专注 {formatMinutes(focusAccumulatedMinutes)}，补水值 {hydrationScore}%，站立值 {standingScore}%</p>
        </div>
        <div class="mini-pet" aria-hidden="true">
          <div class="pet-sprite mini-pet-sprite" style={spriteVars(SPRITE_MINI_WIDTH)}></div>
        </div>
      </section>

      <section class="card-group">
        <article class="feature-card glass row-card focus-card compact-card">
          <div class="row-icon row-icon-focus" aria-hidden="true">⏱</div>
          <div class="row-body">
            <p class="eyebrow">番茄钟</p>
            <h3>{timerLabel}</h3>
            <p class="feature-copy">
              {phase === 'focus'
                ? '专注中'
                : phase === 'break'
                  ? '休息中'
                  : '25 / 5'}
            </p>
          </div>
          <button
            class="round-action"
            type="button"
            aria-label={phase === 'focus' || phase === 'break' ? '重新开始专注' : '开始专注'}
            on:click={startFocus}
          >
            {phase === 'focus' || phase === 'break' ? '↻' : '▶'}
          </button>
          <div class="row-foot">
            <button class="mini-ghost" type="button" on:click={togglePause}>
              {phase === 'paused-focus' || phase === 'paused-break' ? '继续' : '暂停'}
            </button>
            <button class="mini-ghost" type="button" on:click={resetTimer}>重置</button>
            <span class="metric">{config.focusMinutes}/{config.breakMinutes}m</span>
            <div class="stepper compact">
              <button type="button" on:click={() => adjustFocusMinutes(-5)}>-</button>
              <strong>{config.focusMinutes}m</strong>
              <button type="button" on:click={() => adjustFocusMinutes(5)}>+</button>
            </div>
            <div class="stepper compact">
              <button type="button" on:click={() => adjustBreakMinutes(-1)}>-</button>
              <strong>{config.breakMinutes}m</strong>
              <button type="button" on:click={() => adjustBreakMinutes(1)}>+</button>
            </div>
          </div>
        </article>

        <article class:due={waterDue} class="feature-card glass row-card compact-card">
          <div class="row-icon row-icon-water" aria-hidden="true">💧</div>
          <div class="row-body">
            <p class="eyebrow">喝水</p>
            <h3>{waterDue ? '该补水了' : '补水稳定'}</h3>
            <p class="feature-copy">{waterDue ? '现在补一口' : '下一次稍后提醒'}</p>
          </div>
          <button class="round-action" type="button" aria-label="我喝了" on:click={confirmWater}>✓</button>
          <div class="row-foot">
            <span class="metric">{config.waterIntervalMinutes}m</span>
            <div class="stepper compact">
              <button type="button" on:click={() => adjustWaterMinutes(-15)}>-</button>
              <strong>{config.waterIntervalMinutes}m</strong>
              <button type="button" on:click={() => adjustWaterMinutes(15)}>+</button>
            </div>
          </div>
        </article>

        <article class:due={standDue} class="feature-card glass row-card compact-card">
          <div class="row-icon row-icon-stand" aria-hidden="true">🧍</div>
          <div class="row-body">
            <p class="eyebrow">起身</p>
            <h3>{standDue ? '该起来动一下' : '节奏正常'}</h3>
            <p class="feature-copy">{standDue ? '走两分钟再回来' : '后台继续计时'}</p>
          </div>
          <button class="round-action" type="button" aria-label="我起来了" on:click={confirmStand}>✓</button>
          <div class="row-foot">
            <span class="metric">{config.standIntervalMinutes}m</span>
            <div class="stepper compact">
              <button type="button" on:click={() => adjustStandMinutes(-15)}>-</button>
              <strong>{config.standIntervalMinutes}m</strong>
              <button type="button" on:click={() => adjustStandMinutes(15)}>+</button>
            </div>
          </div>
        </article>
      </section>
    </aside>
  </div>
{/if}
