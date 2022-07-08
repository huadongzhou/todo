'use strict'

import { app, protocol, BrowserWindow, screen, ipcMain } from 'electron'
import { createProtocol } from 'vue-cli-plugin-electron-builder/lib'
import installExtension, { VUEJS3_DEVTOOLS } from 'electron-devtools-installer'
import { autoUpdater } from 'electron-updater'

process.env.GH_TOKEN = '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'
console.log('gitPath', process.env.GH_TOKEN)
const isDevelopment = process.env.NODE_ENV !== 'production'

import { initExt, initTray, createAppMenu } from '@/utils/bgExt.js'
import pkg from "../package.json"

let win

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
createAppMenu()

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
    resizable: false,
    minimizable: false,
    maximizable: false,
    skipTaskbar: true,
    //closable: false,
    //show: false,
    transparent: true,
    alwaysOnTop: true,
    useContentSize: true,
    webPreferences: {
      // Use pluginOptions.nodeIntegration, leave this alone
      // See nklayman.github.io/vue-cli-plugin-electron-builder/guide/security.html#node-integration for more info
      nodeIntegration: process.env.ELECTRON_NODE_INTEGRATION,
      contextIsolation: false
    }
  })
  //移动默认位置
  setPosition()
  console.log('创建窗口')
  if (process.env.WEBPACK_DEV_SERVER_URL) {
    // Load the url of the dev server if in development mode
    await win.loadURL(process.env.WEBPACK_DEV_SERVER_URL)
    if (process.env.IS_TEST) win.webContents.openDevTools()
  } else {
    createProtocol('app')
    // Load the index.html when not in development
    win.loadURL('app://./index.html')
    autoUpdater.checkForUpdatesAndNotify()
  }

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
  if (isDevelopment && !process.env.IS_TEST) {
    // Install Vue Devtools
    try {
      await installExtension(VUEJS3_DEVTOOLS)
    } catch (e) {
      console.error('Vue Devtools failed to install:', e.toString())
    }
  }
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
  console.log('定位')
  const displays = screen.getAllDisplays()
  if (displays.length > 1) {
    const standard = displays[0]
    const displayWidth = displays.reduce((t, m, i) => {
      return t += (i == 0 ? m.bounds.width : m.bounds.width / standard.scaleFactor)
    }, 0)
    // console.log('screen', displays, displayWidth)
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
  console.log('处理鼠标事件')
  if (ignore) win.setIgnoreMouseEvents(true, { forward: true })
  else win.setIgnoreMouseEvents(false)
})
ipcMain.handle('hideWindow', event => {
  win.hide()
})

//监听文件更新
function sendStatusToWindow (text) {
  mainWindow.webContents.send('message', text)
}

autoUpdater.on('checking-for-update', () => {
  sendStatusToWindow('Checking for update...')
})

autoUpdater.on('update-available', (info) => {
  sendStatusToWindow('Update available.')
})

autoUpdater.on('update-not-available', (info) => {
  sendStatusToWindow('Update not available.')
})

autoUpdater.on('error', (err) => {
  sendStatusToWindow('Error in auto-updater. ' + err)
})

autoUpdater.on('download-progress', (progressObj) => {
  let log_message = "Download speed: " + progressObj.bytesPerSecond
  log_message = log_message + ' - Downloaded ' + progressObj.percent + '%'
  log_message = log_message + ' (' + progressObj.transferred + "/" + progressObj.total + ')'
  sendStatusToWindow(log_message)
})

autoUpdater.on('update-downloaded', (info) => {
  sendStatusToWindow('Update downloaded')
})