// import { BrowserWindow, screen, ipcMain } from 'electron'

let children = {

}
let screens = {
  update: {
    width: 360,
    height: 240,
    position: ['center', 'center'],
    offset: 20
  }
}
function setPosition (childName) {
  const size = screen.getPrimaryDisplay().workAreaSize
  const child = screens[childName]

  let x = child.position[0] == 'center' ? (size.width - child.width) / 2 : child.position[0] == 'right' ? size.width - child.width - child.offset : child.offset
  let y = child.position[1] == 'center' ? (size.height - child.height) / 2 : child.position[1] == 'bottom' ? size.height - child.height - child.offset : child.offset
  console.log(size, x, y)
  children[childName].setPosition(x, y)
}
function showWindow () {
  if (!win.isVisible()) win.show()
}

export async function createChildWindow (parent, name) {
  // Create the catch browser window.
  children[name] = new BrowserWindow({
    parent,
    width: screens[name].width,
    height: screens[name].height,
    minWidth: screens[name].width,
    minHeight: screens[name].height,
    type: "toolbar",
    frame: false,
    title: name,
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

  let child = children[name]

  if (process.env.WEBPACK_DEV_SERVER_URL) {
    // Load the url of the dev server if in development mode
    await child.loadURL(process.env.WEBPACK_DEV_SERVER_URL + '#/update')
    // if (!process.env.IS_TEST) child.webContents.openDevTools()
  } else {
    createProtocol('app')
    // Load the index.html when not in development
    child.loadURL('app://./index.html#/update')
  }
  //设置位置
  setPosition(name)
}

//监听子窗口事件

// ipcMain.handle('childWin-close', (event, name) => {
//   children[name].close()
// })