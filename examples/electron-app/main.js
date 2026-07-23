// Fixture Electron main process for wrap-swap demos.
// Exercises the API surface wrap-swap's parser detects — it is not a runnable app.
const { app, BrowserWindow, Tray, Menu, ipcMain, globalShortcut, dialog, powerMonitor } =
  require('electron');
const { autoUpdater } = require('electron-updater');

let tray = null;

app.whenReady().then(() => {
  const win = new BrowserWindow({ width: 1024, height: 768 });
  win.loadURL('http://localhost:1420/');

  tray = new Tray('icon.png');
  tray.setContextMenu(Menu.buildFromTemplate([{ role: 'quit' }]));

  globalShortcut.register('CommandOrControl+Shift+K', () => win.show());

  app.setAsDefaultProtocolClient('exampleapp');

  powerMonitor.on('suspend', () => console.log('suspending'));

  autoUpdater.checkForUpdatesAndNotify();
});

ipcMain.handle('pick-file', async () => dialog.showOpenDialog({ properties: ['openFile'] }));
