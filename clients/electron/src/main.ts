/**
 * Alan Electron - Main Process
 * 
 * The main Electron process that creates windows and manages the application lifecycle.
 */

import { app, BrowserWindow, ipcMain, Menu, shell } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import log from 'electron-log';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Configure logging
log.initialize();
log.info('Starting Alan Electron...');

// Keep a global reference of the window object
let mainWindow: BrowserWindow | null = null;

const isDev = process.env.NODE_ENV === 'development';

function createWindow(): void {
  // Create the browser window
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 900,
    minHeight: 600,
    titleBarStyle: 'hiddenInset',
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js'),
    },
    show: false, // Don't show until ready
  });

  // Load the app
  if (isDev) {
    // In development, load from the dev server or file
    mainWindow.loadFile(path.join(__dirname, '../index.html'));
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '../index.html'));
  }

  // Show window when ready
  mainWindow.once('ready-to-show', () => {
    mainWindow?.show();
    
    if (isDev) {
      mainWindow?.webContents.openDevTools();
    }
  });

  // Handle window closed
  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  // Open external links in browser
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url);
    return { action: 'deny' };
  });
}

// Create application menu
function createMenu(): void {
  const template: Electron.MenuItemConstructorOptions[] = [
    {
      label: 'Alan',
      submenu: [
        { role: 'about' },
        { type: 'separator' },
        { role: 'services' },
        { type: 'separator' },
        { role: 'hide' },
        { role: 'hideOthers' },
        { role: 'unhide' },
        { type: 'separator' },
        { role: 'quit' },
      ],
    },
    {
      label: 'File',
      submenu: [
        {
          label: 'New Session',
          accelerator: 'CmdOrCtrl+N',
          click: () => {
            mainWindow?.webContents.send('menu:new-session');
          },
        },
        { type: 'separator' },
        {
          label: 'Close Session',
          accelerator: 'CmdOrCtrl+W',
          click: () => {
            mainWindow?.webContents.send('menu:close-session');
          },
        },
      ],
    },
    {
      label: 'Edit',
      submenu: [
        { role: 'undo' },
        { role: 'redo' },
        { type: 'separator' },
        { role: 'cut' },
        { role: 'copy' },
        { role: 'paste' },
        { role: 'selectAll' },
      ],
    },
    {
      label: 'View',
      submenu: [
        { role: 'reload' },
        { role: 'forceReload' },
        { role: 'toggleDevTools' },
        { type: 'separator' },
        { role: 'actualSize' },
        { role: 'zoomIn' },
        { role: 'zoomOut' },
        { type: 'separator' },
        { role: 'togglefullscreen' },
      ],
    },
    {
      label: 'Window',
      submenu: [
        { role: 'minimize' },
        { role: 'close' },
        { type: 'separator' },
        { role: 'front' },
      ],
    },
    {
      label: 'Help',
      submenu: [
        {
          label: 'Documentation',
          click: () => {
            shell.openExternal('https://github.com/yourusername/alan');
          },
        },
        {
          label: 'Report Issue',
          click: () => {
            shell.openExternal('https://github.com/yourusername/alan/issues');
          },
        },
      ],
    },
  ];

  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);
}

// App event handlers
app.whenReady().then(() => {
  createWindow();
  createMenu();

  app.on('activate', () => {
    // On macOS, re-create window when dock icon is clicked
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  // On macOS, keep app running until user quits explicitly
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

// IPC handlers for communication with renderer process
ipcMain.handle('app:get-version', () => {
  return app.getVersion();
});

ipcMain.handle('app:get-platform', () => {
  return process.platform;
});

// Settings management
const settings = {
  get: (key: string) => {
    // TODO: Implement settings persistence
    return null;
  },
  set: (key: string, value: unknown) => {
    // TODO: Implement settings persistence
    log.info(`Setting ${key} = ${value}`);
  },
};

ipcMain.handle('settings:get', (_event, key: string) => {
  return settings.get(key);
});

ipcMain.handle('settings:set', (_event, key: string, value: unknown) => {
  settings.set(key, value);
});

// Agent daemon connection settings
ipcMain.handle('agentd:get-url', () => {
  return process.env.AGENTD_URL || 'ws://localhost:8090';
});
