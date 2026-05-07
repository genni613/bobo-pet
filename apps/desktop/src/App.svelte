<script lang="ts">
  import { onMount } from 'svelte';
  import spriteSheet from './assets/orange-pet/spritesheet.webp';
  import {
    resolveAnimationState,
    isPausedPhase,
    resolveFacingDirection,
    resolvePetState,
    type AnimationState,
    type FacingDirection,
    type IdleAction,
    type PetState,
    type PetStateOverride,
  } from './lib/pet-state';
  import { defaultConfig, type PetConfig, type TimerPhase } from './lib/types';
  import { loadBootstrap, saveConfig, setPanelVisible } from './lib/desktop';
  import { getCurrentWindow, primaryMonitor, LogicalPosition, LogicalSize } from '@tauri-apps/api/window';

  const isPetView = typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('view') === 'pet';
  const isTauriRuntime = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  const appWindow = isTauriRuntime ? getCurrentWindow() : null;

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
  const RUN_FRAME_VX_THRESHOLD = 18;
  const SPRITE_COLUMNS = 8;
  const SPRITE_ROWS = 9;
  const SPRITE_CELL_WIDTH = 192;
  const SPRITE_CELL_HEIGHT = 208;
  const SPRITE_MAIN_WIDTH = 124;
  const IDLE_WANDER_SPEED = 128;
  const IDLE_WANDER_DURATION_MS = 980;
  const IDLE_WANDER_INTERVAL_MS = 3200;
  const IDLE_WANDER_EDGE_BUFFER = 42;

  const SPRITE_VARIANTS: Record<AnimationState, { row: number; frames: number; speedMs: number }> = {
    idle: { row: 0, frames: 6, speedMs: 180 },
    focusing: { row: 6, frames: 6, speedMs: 180 },
    breaking: { row: 8, frames: 6, speedMs: 140 },
    paused: { row: 0, frames: 6, speedMs: 180 },
    'dragging-left': { row: 2, frames: 8, speedMs: 72 },
    'dragging-right': { row: 1, frames: 8, speedMs: 72 },
    jumping: { row: 4, frames: 5, speedMs: 120 },
    landing: { row: 4, frames: 5, speedMs: 120 },
    'wandering-left': { row: 2, frames: 8, speedMs: 72 },
    'wandering-right': { row: 1, frames: 8, speedMs: 72 },
    'water-reminding': { row: 8, frames: 6, speedMs: 140 },
    'stand-reminding': { row: 8, frames: 6, speedMs: 140 },
    celebrating: { row: 3, frames: 4, speedMs: 140 },
    'hydrated-feedback': { row: 8, frames: 6, speedMs: 140 },
    'recovered-feedback': { row: 8, frames: 6, speedMs: 140 },
    'hover-react': { row: 3, frames: 4, speedMs: 140 },
    ready: { row: 0, frames: 6, speedMs: 180 }
  };

  type PanelView = 'dashboard' | 'stats';

  let viewportWidth = isPetView ? PET_WINDOW_WIDTH : 252;
  let viewportHeight = isPetView ? PET_WINDOW_HEIGHT : 492;
  let config: PetConfig = { ...defaultConfig };
  let booted = false;

  let panelOpen = !isPetView;
  let panelView: PanelView = 'dashboard';
  let phase: TimerPhase = 'idle';
  let timerEndsAt: number | null = null;
  let remainingMs = defaultConfig.remainingMs;
  let activeFocusLengthMs = defaultConfig.activeFocusLengthMs;
  let focusSessions = 0;
  let petState: PetState = 'idle';
  let petStateOverride: PetStateOverride = null;
  let petStateOverrideUntil = 0;
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
  let phaseLabel = '';
  let focusCaption = '点击开始后进入专注计时。';
  let waterCaption = '按固定节奏补水，别让叶冠先蔫下来。';
  let standCaption = '每隔一段时间起身，让宠物也跟着舒展。';

  let petX = 300;
  let petY = 0;
  let petVx = 0;
  let petVy = 0;
  let petScaleX = 1;
  let petScaleY = 1;
  let petRotation = 0;
  let facingDirection: FacingDirection = 'right';
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
  let animationState: AnimationState = 'idle';
  let spriteSpec = SPRITE_VARIANTS.idle;
  let lastHoverJumpAt = 0;

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
  let nextIdleWanderDirection: FacingDirection = 'right';
  let idleAction: IdleAction = 'none';
  let idleActionEndsAt = 0;
  let nextIdleActionAt = Date.now() + 3000;

  const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
  const lerp = (from: number, to: number, amount: number) => from + (to - from) * amount;
  const floorY = () => viewportHeight - PET_HALF_Y - FLOOR_PADDING;

  const spriteVars = (width: number, row: number, frame: number) => {
    const height = (width * SPRITE_CELL_HEIGHT) / SPRITE_CELL_WIDTH;
    return [
      `--sprite-sheet:url(${spriteSheet})`,
      `--sprite-columns:${SPRITE_COLUMNS}`,
      `--sprite-rows:${SPRITE_ROWS}`,
      `--sprite-display-cell-width:${width}px`,
      `--sprite-display-cell-height:${height}px`,
      `--sprite-row:${row}`,
      `--sprite-frame:${frame}`,
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

  const setPetStateOverride = (nextState: Exclude<PetStateOverride, null>, stickyMs = 0) => {
    petStateOverride = nextState;
    petStateOverrideUntil = stickyMs > 0 ? Date.now() + stickyMs : 0;
  };

  const refreshPetStateOverride = () => {
    if (petStateOverrideUntil > Date.now()) {
      return;
    }
    petStateOverride = null;
    petStateOverrideUntil = 0;
  };

  const updateViewport = async () => {
    if (isPetView) {
      if (isTauriRuntime) {
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
    if (!isPetView || !booted || !appWindow) {
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
    refreshPetStateOverride();

    if (isPetView) {
      if (appWindow) {
        await appWindow.setSize(new LogicalSize(PET_WINDOW_WIDTH, PET_WINDOW_HEIGHT));
      }
      void syncPetWindow();
    }
  };

  const startFocus = async () => {
    phase = 'focus';
    activeFocusLengthMs = config.focusMinutes * 60_000;
    remainingMs = activeFocusLengthMs;
    timerEndsAt = Date.now() + remainingMs;
    refreshPetStateOverride();
    await persistConfig();
  };

  const togglePause = async () => {
    if (phase === 'focus' || phase === 'break') {
      remainingMs = Math.max(0, (timerEndsAt ?? Date.now()) - Date.now());
      phase = phase === 'focus' ? 'paused-focus' : 'paused-break';
      timerEndsAt = null;
      await persistConfig();
      return;
    }
    if (phase === 'paused-focus' || phase === 'paused-break') {
      timerEndsAt = Date.now() + remainingMs;
      phase = phase === 'paused-focus' ? 'focus' : 'break';
      refreshPetStateOverride();
      await persistConfig();
    }
  };

  const resetTimer = async () => {
    phase = 'idle';
    timerEndsAt = null;
    remainingMs = config.focusMinutes * 60_000;
    activeFocusLengthMs = remainingMs;
    refreshPetStateOverride();
    await persistConfig();
  };

  const confirmWater = async () => {
    config.lastWaterAt = Date.now();
    setPetStateOverride('hydrated-feedback', 3200);
    await persistConfig();
  };

  const confirmStand = async () => {
    config.lastStandAt = Date.now();
    setPetStateOverride('recovered-feedback', 3200);
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
    refreshPetStateOverride();
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
    facingDirection = resolveFacingDirection(facingDirection, petVx);
    petRotation = 0;
    petScaleX = clamp(1 + Math.max(-petVy, 0) / 3200, 1, 1.12);
    petScaleY = clamp(1 - Math.abs(petVx) / 5200, 0.9, 1.02);
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

    refreshPetStateOverride();
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
    panelView = 'dashboard';
    await persistConfig();
    await setPanelVisible(true);
  };

  const closePanel = async () => {
    if (isPetView) {
      return;
    }
    panelOpen = false;
    panelView = 'dashboard';
    await persistConfig();
    await setPanelVisible(false);
  };

  const togglePanelView = () => {
    panelView = panelView === 'dashboard' ? 'stats' : 'dashboard';
  };

  const hoverJump = () => {
    if (!isPetView || isDragging || isAirborne) return;
    const nowMs = Date.now();
    if (nowMs - lastHoverJumpAt < 2200) return;
    lastHoverJumpAt = nowMs;
    petVy = -260;
    isAirborne = true;
    idleAction = 'wave';
    idleActionEndsAt = nowMs + 900;
    setPetStateOverride('hover-react', 1800);
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
      petRotation = 0;

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
          setPetStateOverride('landing', 220);
        } else {
          petVy = 0;
          petVx = 0;
          isAirborne = false;
          refreshPetStateOverride();
        }
      }
      void syncPetWindow();
    } else {
      petY = floor;
      if (idleAction === 'wander-left' || idleAction === 'wander-right') {
        petX += petVx * dt;
        petX = clamp(petX, PET_HALF_X + SIDE_PADDING, viewportWidth - PET_HALF_X - SIDE_PADDING);
        petRotation = 0;
        const stride = Math.sin(now / 65);
        petScaleX = lerp(petScaleX, 0.97 + Math.abs(stride) * 0.07, 0.2);
        petScaleY = lerp(petScaleY, 1.01 - Math.abs(stride) * 0.08, 0.2);
        void syncPetWindow();
      } else {
        petRotation = lerp(petRotation, 0, 0.12);
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
          setPetStateOverride('celebrating', 3800);
        } else {
          phase = 'idle';
          timerEndsAt = null;
          remainingMs = config.focusMinutes * 60_000;
          setPetStateOverride('ready', 2800);
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
        petScaleX = 1.04;
        petScaleY = 0.96;
      }
      idleAction = 'none';
      refreshPetStateOverride();
    }

    if (idleAction === 'none' && now > nextIdleActionAt) {
      const leftBound = PET_HALF_X + SIDE_PADDING;
      const rightBound = viewportWidth - PET_HALF_X - SIDE_PADDING;
      let dir = nextIdleWanderDirection;

      if (petX <= leftBound + IDLE_WANDER_EDGE_BUFFER) {
        dir = 'right';
      } else if (petX >= rightBound - IDLE_WANDER_EDGE_BUFFER) {
        dir = 'left';
      }

      idleAction = dir === 'left' ? 'wander-left' : 'wander-right';
      petVx = dir === 'left' ? -IDLE_WANDER_SPEED : IDLE_WANDER_SPEED;
      facingDirection = dir;
      nextIdleWanderDirection = dir === 'left' ? 'right' : 'left';
      petScaleX = 0.98;
      petScaleY = 1.02;
      idleActionEndsAt = now + IDLE_WANDER_DURATION_MS;
      nextIdleActionAt = now + IDLE_WANDER_INTERVAL_MS;
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
      refreshPetStateOverride();
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
        : isPausedPhase(phase)
          ? '已暂停'
          : '';
  $: focusCaption =
    phase === 'focus'
      ? '当前处于专注状态。'
      : phase === 'break'
        ? '当前处于休息状态。'
        : '点击开始后进入专注计时。';
  $: waterCaption = waterDue
    ? '补水时间到了。'
    : `每 ${config.waterIntervalMinutes} 分钟检查一次补水。`;
  $: standCaption = standDue
    ? '该起身活动一下了。'
    : `每 ${config.standIntervalMinutes} 分钟提醒一次起身。`;
  $: petState = resolvePetState({
    idleAction,
    isAirborne,
    isDragging,
    overrideState: petStateOverride,
    phase,
    standDue,
    waterDue
  });
  $: ({ animationState, facingDirection } = resolveAnimationState({
    facingDirection,
    idleAction,
    petState,
    petVx,
    runFrameVxThreshold: RUN_FRAME_VX_THRESHOLD
  }));
  $: spriteSpec = SPRITE_VARIANTS[animationState];
  $: spriteFrame =
    animationState === 'idle' || animationState === 'paused' || animationState === 'ready'
      ? idleBlinkFrame(now)
      : Math.floor(now / spriteSpec.speedMs) % spriteSpec.frames;
</script>

<svelte:window on:pointermove={pointerMove} on:pointerup={pointerUp} on:pointercancel={pointerUp} />

{#if isPetView}
  <div class="shell pet-shell">
    <button
      class={`pet pet-window pet-${petState}`}
      type="button"
      aria-label="桌面宠物"
      on:pointerenter={hoverJump}
      on:pointerdown={pointerDown}
      on:click={openPanel}
      style={`--pet-scale-x:${petScaleX}; --pet-scale-y:${petScaleY}; --pet-rotate:${petRotation}deg;`}
    >
      <div class="pet-frame" aria-hidden="true">
        <div class="pet-sprite main-pet-sprite" style={spriteVars(SPRITE_MAIN_WIDTH, spriteSpec.row, spriteFrame)}></div>
      </div>
    </button>
  </div>
{:else}
  <div class="shell panel-shell">
    <div class="atmosphere"></div>
    <div class="orb orb-a"></div>
    <div class="orb orb-b"></div>

    <aside class:open={panelOpen} class="panel">
      <div class="panel-content">
        <div class="panel-head">
          <div class="panel-brand">
            <div class="panel-mark" aria-hidden="true">
              <div class="pet-sprite panel-mark-sprite" style={spriteVars(28, spriteSpec.row, spriteFrame)}></div>
            </div>
            <div class="panel-title-group">
              <h1>卜卜</h1>
            </div>
          </div>
          <div class="panel-actions">
            <button
              class="ghost panel-view-toggle"
              type="button"
              on:click={togglePanelView}
              aria-label={panelView === 'dashboard' ? '查看统计' : '返回首页'}
            >
              {#if panelView === 'dashboard'}
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <rect x="2" y="9" width="2" height="5" rx="1"></rect>
                  <rect x="7" y="6" width="2" height="8" rx="1"></rect>
                  <rect x="12" y="3" width="2" height="11" rx="1"></rect>
                </svg>
              {:else}
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M9.8 3.2 6 7h7v2H6l3.8 3.8-1.4 1.4L2 8l6.4-6.2 1.4 1.4Z"></path>
                </svg>
              {/if}
              <span>{panelView === 'dashboard' ? '统计' : '首页'}</span>
            </button>
            <button class="ghost panel-close" type="button" on:click={closePanel} aria-label="收起面板">
              <span class="panel-close-icon" aria-hidden="true"></span>
              <span>收起</span>
            </button>
          </div>
        </div>

        {#if panelView === 'dashboard'}
          <section class="card-group">
            <article class="feature-card focus-card reminder-card">
              <div class="row-icon row-icon-focus" aria-hidden="true">
                <svg class="pixel-icon" viewBox="0 0 16 16" shape-rendering="crispEdges">
                  <rect x="6" y="1" width="4" height="2"></rect>
                  <rect x="5" y="3" width="6" height="1"></rect>
                  <rect x="4" y="4" width="8" height="1"></rect>
                  <rect x="3" y="5" width="10" height="1"></rect>
                  <rect x="2" y="6" width="12" height="6"></rect>
                  <rect x="3" y="12" width="10" height="1"></rect>
                  <rect x="4" y="13" width="8" height="1"></rect>
                  <rect x="5" y="14" width="6" height="1"></rect>
                  <rect x="7" y="7" width="1" height="3" class="pixel-ink"></rect>
                  <rect x="8" y="8" width="2" height="1" class="pixel-ink"></rect>
                  <rect x="10" y="7" width="1" height="1" class="pixel-ink"></rect>
                </svg>
              </div>
              <div class="row-body">
                <span class="timer-inline">{timerLabel}</span>
                <span class="focus-chip">{phaseLabel || '准备就绪'}</span>
              </div>
              <button
                class="round-action"
                type="button"
                aria-label={phase === 'focus' || phase === 'break' ? '暂停' : phase === 'idle' ? '开始专注' : '继续'}
                on:click={phase === 'idle' ? startFocus : togglePause}
              >
                {phase === 'focus' || phase === 'break' ? '⏸' : '▶'}
              </button>
              <div class="row-foot row-foot-focus">
                <div class="timer-settings">
                  <div class="timer-setting timer-setting-focus">
                    <span class="timer-setting-label">专注</span>
                    <div class="stepper">
                      <button type="button" on:click={() => adjustFocusMinutes(-5)}>-</button>
                      <strong>{config.focusMinutes}m</strong>
                      <button type="button" on:click={() => adjustFocusMinutes(5)}>+</button>
                    </div>
                  </div>
                  <div class="timer-setting timer-setting-break">
                    <span class="timer-setting-label">休息</span>
                    <div class="stepper">
                      <button type="button" on:click={() => adjustBreakMinutes(-1)}>-</button>
                      <strong>{config.breakMinutes}m</strong>
                      <button type="button" on:click={() => adjustBreakMinutes(1)}>+</button>
                    </div>
                  </div>
                </div>
              </div>
            </article>

            <article class:due={waterDue} class="feature-card reminder-card water-card">
              <div class="row-icon row-icon-water" aria-hidden="true">
                <svg class="pixel-icon" viewBox="0 0 16 16" shape-rendering="crispEdges">
                  <rect x="7" y="1" width="2" height="2"></rect>
                  <rect x="6" y="3" width="4" height="1"></rect>
                  <rect x="5" y="4" width="6" height="2"></rect>
                  <rect x="4" y="6" width="8" height="3"></rect>
                  <rect x="5" y="9" width="6" height="2"></rect>
                  <rect x="6" y="11" width="4" height="2"></rect>
                  <rect x="7" y="13" width="2" height="1"></rect>
                  <rect x="10" y="5" width="1" height="2" class="pixel-highlight"></rect>
                  <rect x="9" y="4" width="1" height="1" class="pixel-highlight"></rect>
                </svg>
              </div>
              <div class="row-body">
                <h3>{waterDue ? '补水提醒' : '补水正常'}</h3>
                <p class="row-copy">{waterCaption}</p>
              </div>
              <button class="round-action" type="button" aria-label="我喝了" on:click={confirmWater}>✓</button>
              <div class="row-foot">
                <div class="stepper">
                  <button type="button" on:click={() => adjustWaterMinutes(-15)}>-</button>
                  <strong>{config.waterIntervalMinutes}m</strong>
                  <button type="button" on:click={() => adjustWaterMinutes(15)}>+</button>
                </div>
              </div>
            </article>

            <article class:due={standDue} class="feature-card reminder-card stand-card">
              <div class="row-icon row-icon-stand" aria-hidden="true">
                <svg class="pixel-icon" viewBox="0 0 16 16" shape-rendering="crispEdges">
                  <rect x="7" y="2" width="2" height="7"></rect>
                  <rect x="5" y="4" width="2" height="2"></rect>
                  <rect x="9" y="4" width="2" height="2"></rect>
                  <rect x="6" y="1" width="1" height="1"></rect>
                  <rect x="9" y="1" width="1" height="1"></rect>
                  <rect x="5" y="9" width="6" height="2"></rect>
                  <rect x="4" y="11" width="3" height="2"></rect>
                  <rect x="9" y="11" width="3" height="2"></rect>
                  <rect x="3" y="13" width="4" height="1"></rect>
                  <rect x="9" y="13" width="4" height="1"></rect>
                  <rect x="7" y="0" width="2" height="1" class="pixel-highlight"></rect>
                </svg>
              </div>
              <div class="row-body">
                <h3>{standDue ? '起身提醒' : '活动正常'}</h3>
                <p class="row-copy">{standCaption}</p>
              </div>
              <button class="round-action" type="button" aria-label="我起来了" on:click={confirmStand}>✓</button>
              <div class="row-foot">
                <div class="stepper">
                  <button type="button" on:click={() => adjustStandMinutes(-15)}>-</button>
                  <strong>{config.standIntervalMinutes}m</strong>
                  <button type="button" on:click={() => adjustStandMinutes(15)}>+</button>
                </div>
              </div>
            </article>
          </section>
        {:else}
          <section class="stats-card">
            <div class="stats-head">
              <h2>今日统计</h2>
              <p class="stats-note">{phaseLabel ? `${phaseLabel} · ${timerLabel}` : timerLabel}</p>
            </div>

            <div class="stats-grid">
              <article class="stat-tile">
                <span>专注次数</span>
                <strong>{focusSessions}</strong>
              </article>
              <article class="stat-tile">
                <span>累计专注</span>
                <strong>{formatMinutes(focusAccumulatedMinutes)}</strong>
              </article>
              <article class="stat-tile">
                <span>补水进度</span>
                <strong>{hydrationScore}%</strong>
              </article>
              <article class="stat-tile">
                <span>起身进度</span>
                <strong>{standingScore}%</strong>
              </article>
            </div>

            <div class="stats-strip">
              <span class="status-chip">{config.focusMinutes}m 专注</span>
              <span class="status-chip">{config.breakMinutes}m 休息</span>
            </div>
          </section>
        {/if}
      </div>
    </aside>
  </div>
{/if}
