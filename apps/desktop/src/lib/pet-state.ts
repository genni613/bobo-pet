import type { TimerPhase } from './types';

export type FacingDirection = 'left' | 'right';

export type AnimationState =
  | 'idle'
  | 'focusing'
  | 'breaking'
  | 'paused'
  | 'dragging-left'
  | 'dragging-right'
  | 'jumping'
  | 'landing'
  | 'wandering-left'
  | 'wandering-right'
  | 'water-reminding'
  | 'stand-reminding'
  | 'celebrating'
  | 'hydrated-feedback'
  | 'recovered-feedback'
  | 'hover-react'
  | 'ready';

export type IdleAction = 'none' | 'wander-left' | 'wander-right' | 'wave' | 'hop';

export type PetState =
  | 'idle'
  | 'focusing'
  | 'breaking'
  | 'paused'
  | 'dragging'
  | 'jumping'
  | 'landing'
  | 'wandering'
  | 'water-reminding'
  | 'stand-reminding'
  | 'celebrating'
  | 'hydrated-feedback'
  | 'recovered-feedback'
  | 'hover-react'
  | 'ready';

export type PetStateOverride =
  | 'landing'
  | 'celebrating'
  | 'hydrated-feedback'
  | 'recovered-feedback'
  | 'hover-react'
  | 'ready'
  | null;

export interface PetStateContext {
  idleAction: IdleAction;
  isAirborne: boolean;
  isDragging: boolean;
  overrideState: PetStateOverride;
  phase: TimerPhase;
  standDue: boolean;
  waterDue: boolean;
}

export interface AnimationResolutionContext {
  facingDirection: FacingDirection;
  idleAction: IdleAction;
  petState: PetState;
  petVx: number;
  runFrameVxThreshold: number;
}

export const isPausedPhase = (phase: TimerPhase) =>
  phase === 'paused-focus' || phase === 'paused-break';

export const resolveFacingDirection = (
  current: FacingDirection,
  vx: number,
  threshold = 8
): FacingDirection => {
  if (vx > threshold) {
    return 'right';
  }
  if (vx < -threshold) {
    return 'left';
  }
  return current;
};

export const resolvePetState = ({
  idleAction,
  isAirborne,
  isDragging,
  overrideState,
  phase,
  standDue,
  waterDue
}: PetStateContext): PetState => {
  if (isDragging) {
    return 'dragging';
  }
  if (isAirborne) {
    return overrideState === 'landing' ? 'landing' : 'jumping';
  }
  if (isPausedPhase(phase)) {
    return 'paused';
  }
  if (overrideState) {
    return overrideState;
  }
  if (phase === 'focus') {
    return 'focusing';
  }
  if (phase === 'break') {
    return 'breaking';
  }
  if (waterDue) {
    return 'water-reminding';
  }
  if (standDue) {
    return 'stand-reminding';
  }
  if (idleAction === 'wander-left' || idleAction === 'wander-right') {
    return 'wandering';
  }
  return 'idle';
};

const directionalAnimation = (
  direction: FacingDirection,
  left: AnimationState,
  right: AnimationState
): AnimationState => (direction === 'right' ? right : left);

const shouldUseDirectionalGroundAnimation = (
  petState: PetState,
  petVx: number,
  runFrameVxThreshold: number
) => {
  if (Math.abs(petVx) <= runFrameVxThreshold) {
    return false;
  }

  switch (petState) {
    case 'idle':
    case 'ready':
    case 'paused':
    case 'focusing':
    case 'breaking':
    case 'water-reminding':
    case 'stand-reminding':
      return true;
    default:
      return false;
  }
};

export const resolveAnimationState = ({
  facingDirection: currentDirection,
  idleAction,
  petState,
  petVx,
  runFrameVxThreshold
}: AnimationResolutionContext): { animationState: AnimationState; facingDirection: FacingDirection } => {
  let facingDirection = currentDirection;

  if (
    petState === 'dragging' ||
    petState === 'wandering' ||
    ((petState === 'idle' || petState === 'ready') && Math.abs(petVx) > runFrameVxThreshold)
  ) {
    facingDirection = resolveFacingDirection(facingDirection, petVx);
  }

  if (idleAction === 'wander-left') {
    facingDirection = 'left';
  } else if (idleAction === 'wander-right') {
    facingDirection = 'right';
  }

  if (shouldUseDirectionalGroundAnimation(petState, petVx, runFrameVxThreshold)) {
    return {
      animationState: directionalAnimation(facingDirection, 'wandering-left', 'wandering-right'),
      facingDirection
    };
  }

  switch (petState) {
    case 'dragging':
      return {
        animationState: directionalAnimation(facingDirection, 'dragging-left', 'dragging-right'),
        facingDirection
      };
    case 'jumping':
      return { animationState: 'jumping', facingDirection };
    case 'landing':
      return { animationState: 'landing', facingDirection };
    case 'wandering':
      return {
        animationState: directionalAnimation(facingDirection, 'wandering-left', 'wandering-right'),
        facingDirection
      };
    case 'celebrating':
      return { animationState: 'celebrating', facingDirection };
    case 'focusing':
      return { animationState: 'focusing', facingDirection };
    case 'breaking':
      return { animationState: 'breaking', facingDirection };
    case 'paused':
      return { animationState: 'paused', facingDirection };
    case 'water-reminding':
      return { animationState: 'water-reminding', facingDirection };
    case 'stand-reminding':
      return { animationState: 'stand-reminding', facingDirection };
    case 'hydrated-feedback':
      return { animationState: 'hydrated-feedback', facingDirection };
    case 'recovered-feedback':
      return { animationState: 'recovered-feedback', facingDirection };
    case 'hover-react':
      return { animationState: 'hover-react', facingDirection };
    case 'ready':
      return { animationState: 'ready', facingDirection };
    case 'idle':
      return { animationState: 'idle', facingDirection };
  }
};
