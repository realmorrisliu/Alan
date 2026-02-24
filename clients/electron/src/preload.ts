/**
 * Alan Electron - Preload Script
 * 
 * This script runs in the renderer process before the web page loads.
 * It exposes a safe API for the renderer to communicate with the main process.
 */

import { contextBridge, ipcRenderer } from 'electron';

// Expose API to renderer process
contextBridge.exposeInMainWorld('electronAPI', {
  // App info
  getVersion: () => ipcRenderer.invoke('app:get-version'),
  getPlatform: () => ipcRenderer.invoke('app:get-platform'),
  
  // Settings
  getSetting: (key: string) => ipcRenderer.invoke('settings:get', key),
  setSetting: (key: string, value: unknown) => ipcRenderer.invoke('settings:set', key, value),
  
  // Agent daemon
  getAgentdUrl: () => ipcRenderer.invoke('agentd:get-url'),
  
  // Menu events
  onMenuNewSession: (callback: () => void) => {
    ipcRenderer.on('menu:new-session', callback);
    return () => ipcRenderer.removeListener('menu:new-session', callback);
  },
  onMenuCloseSession: (callback: () => void) => {
    ipcRenderer.on('menu:close-session', callback);
    return () => ipcRenderer.removeListener('menu:close-session', callback);
  },
});

// Type definitions for TypeScript
declare global {
  interface Window {
    electronAPI: {
      getVersion: () => Promise<string>;
      getPlatform: () => Promise<string>;
      getSetting: (key: string) => Promise<unknown>;
      setSetting: (key: string, value: unknown) => Promise<void>;
      getAgentdUrl: () => Promise<string>;
      onMenuNewSession: (callback: () => void) => () => void;
      onMenuCloseSession: (callback: () => void) => () => void;
    };
  }
}

export {};
