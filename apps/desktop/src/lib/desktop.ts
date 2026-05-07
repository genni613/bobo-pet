import { invoke } from '@tauri-apps/api/core';
import { defaultConfig, type AppBootstrap, type PetConfig } from './types';

const STORAGE_KEY = 'pet-element-config';

const isTauriRuntime = () =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function loadBootstrap(): Promise<AppBootstrap> {
  if (isTauriRuntime()) {
    return invoke<AppBootstrap>('load_bootstrap');
  }

  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) {
    return { config: defaultConfig, platform: 'browser' };
  }

  return {
    config: { ...defaultConfig, ...(JSON.parse(raw) as Partial<PetConfig>) },
    platform: 'browser'
  };
}

export async function saveConfig(config: PetConfig): Promise<void> {
  if (isTauriRuntime()) {
    await invoke('save_config', { config });
    return;
  }

  localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
}

export async function setPanelVisible(visible: boolean): Promise<void> {
  if (isTauriRuntime()) {
    await invoke('set_panel_visible', { visible });
    return;
  }

  const raw = localStorage.getItem(STORAGE_KEY);
  const current = raw ? ({ ...defaultConfig, ...(JSON.parse(raw) as Partial<PetConfig>) } satisfies PetConfig) : defaultConfig;
  localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...current, panelOpen: visible }));
}
