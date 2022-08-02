'use strict'

import path from 'path'
import { app, protocol, BrowserWindow, screen, ipcMain } from 'electron'
import { createProtocol } from 'vue-cli-plugin-electron-builder/lib'
// import installExtension, { VUEJS3_DEVTOOLS } from 'electron-devtools-installer'
import { autoUpdater } from 'electron-updater'



import log from '@/utils/log'
import { initExt, initTray } from '@/utils/bgExt.js'
import { checkVersion, hotVersion, allVersion, reloadVersion } from '@/utils/updater.js'
import pkg from "../package.json"

// console.log('gitPath', process.env.GH_TOKEN)
const isDevelopment = process.env.NODE_ENV !== 'production'

let win

log('当前环境：', process.env.NODE_ENV)
//判断实例是否被打开
if (app.requestSingleInstanceLock()) {
  app.on("second-instance", (event, commandLine, workingDirectory) => {
    if (win) {
      setPosition()
    }
  })
} else {
  app.quit()
}
// Scheme must be registered before the app is ready
protocol.registerSchemesAsPrivileged([
  { scheme: 'app', privileges: { secure: true, standard: true } }
])

async function createWindow () {
  // Create the browser window.
  win = new BrowserWindow({
    width: 320,
    height: 290,
    minWidth: 320,
    minHeight: 290,
    type: "toolbar",
    frame: false,
    title: pkg.name,
    // resizable: false,
    minimizable: false,
    maximizable: false,
    skipTaskbar: true,
    //closable: false,
    //show: false,
    transparent: true,
    alwaysOnTop: true,
    useContentSize: true,
    webPreferences: {
      // webSecurity: false,
      // Use pluginOptions.nodeIntegration, leave this alone
      // See nklayman.github.io/vue-cli-plugin-electron-builder/guide/security.html#node-integration for more info
      nodeIntegration: process.env.ELECTRON_NODE_INTEGRATION,
      contextIsolation: false
    }
  })
  //移动默认位置
  setPosition()

  log('创建窗口', process.env.WEBPACK_DEV_SERVER_URL)
  if (process.env.WEBPACK_DEV_SERVER_URL) {
    // Load the url of the dev server if in development mode
    await win.loadURL(process.env.WEBPACK_DEV_SERVER_URL)
    if (!process.env.IS_TEST) win.webContents.openDevTools()
  } else {
    createProtocol('app')
    // Load the index.html when not in development
    win.loadURL('app://./index.html')
    // autoUpdater.checkForUpdatesAndNotify()
    // checkVersion(relativePath)
  }

  // createChildWindow(win, 'update')
  //屏蔽windows原生右键菜单
  if (process.platform === "win32") {
    //int WM_INITMENU = 0x116;
    //当一个下拉菜单或子菜单将要被激活时发送此消息，它允许程序在它显示前更改菜单，而不要改变全部
    win.hookWindowMessage(278, function (e) {
      win.setEnabled(false) //窗口禁用

      setTimeout(() => {
        win.setEnabled(true) //窗口启用
      }, 100) //延时太快会立刻启用，太慢会妨碍窗口其他操作，可自行测试最佳时间

      return true
    })
  }
  win.on("closed", () => {
    win = null
  })
}

//闪烁问题-控制Chromium行为设计
app.commandLine.appendSwitch("wm-window-animations-disabled")

// Quit when all windows are closed.
app.on('window-all-closed', () => {
  // On macOS it is common for applications and their menu bar
  // to stay active until the user quits explicitly with Cmd + Q
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('activate', () => {
  // On macOS it's common to re-create a window in the app when the
  // dock icon is clicked and there are no other windows open.
  if (BrowserWindow.getAllWindows().length === 0) winInit()
})

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.on('ready', async () => {
  // if (isDevelopment && !process.env.IS_TEST) {
  //   // Install Vue Devtools
  //   try {
  //     await installExtension(VUEJS3_DEVTOOLS)
  //   } catch (e) {
  //     console.error('Vue Devtools failed to install:', e.toString())
  //   }
  // }
  // createWindow()
  winInit()
})
function winInit () {
  //初始化拓展
  initExt()

  //初始化视口
  createWindow()

  //初始化托盘
  initTray(showWindow)
}
// Exit cleanly on request from parent process in development mode.
if (isDevelopment) {
  if (process.platform === 'win32') {
    process.on('message', (data) => {
      if (data === 'graceful-exit') {
        app.quit()
      }
    })
  } else {
    process.on('SIGTERM', () => {
      app.quit()
    })
  }
}


function setPosition () {
  const displays = screen.getAllDisplays()

  // log('定位', displays)
  if (displays.length > 1) {
    const standard = displays[0]
    const displayWidth = displays.reduce((t, m, i) => {
      return t += (i == 0 ? m.bounds.width : m.bounds.width / standard.scaleFactor)
    }, 0)
    // log('screen', displays, displayWidth)
    win.setPosition(displayWidth - 350, 120)
  } else {
    const size = screen.getPrimaryDisplay().workAreaSize
    const winSize = win.getSize()
    win.setPosition(size.width - winSize[0] - 30, 120)
  }
}
function showWindow () {
  if (!win.isVisible()) win.show()
}


//监听进程事件
ipcMain.handle('setIgnoreMouseEvents', (event, ignore) => {
  log('处理鼠标事件')
  if (ignore) win.setIgnoreMouseEvents(true, { forward: true })
  else win.setIgnoreMouseEvents(false)
})
ipcMain.handle('hideWindow', event => {
  win.hide()
})
//检查版本
ipcMain.handle('checkVersion', async event => {
  const version = await checkVersion()
  log('版本查询', version)
  return version
})
//热更新
ipcMain.handle('hotVersion', async (event, version) => {
  const result = await hotVersion(version)
  log('热更新状态', result)
  return result
})
//全量更新
ipcMain.handle('allVersion', event => {
  allVersion()
})
//重启应用
ipcMain.handle('reloadVersion', event => {
  reloadVersion()
})


//向渲染器发送事件
//检查更新
export function menuCheckVersion () {
  win.webContents.send('checkVersion')
}
//监听文件更新
function sendStatusToWindow (text) {
  win.webContents.send('message', text)
}


autoUpdater.on('checking-for-update', () => {
  log('Checking for update...')
  sendStatusToWindow('Checking for update...')
})

autoUpdater.on('update-available', (info) => {
  log('Update available.')
  sendStatusToWindow('Update available.')
})

autoUpdater.on('update-not-available', (info) => {
  log('Update not available.')
  sendStatusToWindow('Update not available.')
})

autoUpdater.on('error', (err) => {
  log('Error in auto-updater')
  sendStatusToWindow('Error in auto-updater. ' + err)
})

autoUpdater.on('download-progress', (progressObj) => {
  log('Download speed')
  let log_message = "Download speed: " + progressObj.bytesPerSecond
  log_message = log_message + ' - Downloaded ' + progressObj.percent + '%'
  log_message = log_message + ' (' + progressObj.transferred + "/" + progressObj.total + ')'
  sendStatusToWindow(log_message)
})

autoUpdater.on('update-downloaded', (info) => {
  log('Update downloaded')
  autoUpdater.quitAndInstall()
  sendStatusToWindow('Update downloaded')
})