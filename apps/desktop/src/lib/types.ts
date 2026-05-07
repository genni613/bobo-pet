export type TimerPhase = 'idle' | 'focus' | 'break' | 'paused-focus' | 'paused-break';

export interface PetConfig {
  focusMinutes: number;
  breakMinutes: number;
  waterIntervalMinutes: number;
  standIntervalMinutes: number;
  petX: number;
  petY: number;
  panelOpen: boolean;
  phase: TimerPhase;
  remainingMs: number;
  activeFocusLengthMs: number;
  focusSessions: number;
  lastWaterAt: number;
  lastStandAt: number;
}

export interface AppBootstrap {
  config: PetConfig;
  platform: string;
}

export const defaultConfig: PetConfig = {
  focusMinutes: 25,
  breakMinutes: 5,
  waterIntervalMinutes: 45,
  standIntervalMinutes: 60,
  petX: 300,
  petY: 620,
  panelOpen: false,
  phase: 'idle',
  remainingMs: 25 * 60_000,
  activeFocusLengthMs: 25 * 60_000,
  focusSessions: 0,
  lastWaterAt: Date.now(),
  lastStandAt: Date.now()
};
