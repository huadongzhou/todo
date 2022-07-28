
import { app, ipcMain, Tray, Menu } from "electron"
import DB from "@/db"

import path from "path"
import pkg from "../../package.json"

import { checkVersion } from '../utils/updater'

const userPath = app.getPath("userData")

//外置防止内存清理释放托盘
let tray

ipcMain.handle('getuserPath', () => {
  return userPath
})
//初始化拓展
export function initExt () {
  //初始化数据库
  DB.initDB(userPath)

  const firstRun = DB.get("settings.firstRun")
  console.log('firstRun', firstRun)
  if (firstRun) {
    //首次运行设置开机自启动
    setOpenAtLogin(true)
    DB.set("settings.firstRun", false)
  }
}

//初始化托盘
export function initTray (setPosition) {
  //设置托盘图标
  tray = new Tray(path.join(__static, process.platform != "darwin" ? "./tray.png" : "./tray-mac.png"))
  //设置托盘参数
  const trayItems = Menu.buildFromTemplate([
    {
      label: "开机自启动",
      type: "checkbox",
      checked: getOpenAtLogin,
      click () {
        const openAtLogin = getOpenAtLogin()
        setOpenAtLogin(!openAtLogin)
      }
    },
    {
      label: "版本V " + pkg.version,
      type: "normal",
    },
    {
      label: "检查更新",
      type: "normal",
      click () {
        //客户端更新  热更新
        checkVersion()
      }
    },
    {
      label: "退出",
      role: "quit"
    }
  ])

  //设置菜单项
  tray.setContextMenu(trayItems)
  //设置项目名
  tray.setToolTip(pkg.name)
  //设置点击事件
  tray.on("click", (event, bounds, position) => {
    setPosition()
  })
}
//设置开机自启动
function setOpenAtLogin (openAtLogin) {
  if (app.isPackaged) {
    app.setLoginItemSettings({
      openAtLogin: openAtLogin
    })
  } else {
    app.setLoginItemSettings({
      openAtLogin: openAtLogin,
      openAsHidden: false,
      path: process.execPath,
      args: [path.resolve(process.argv[1])]
    })
  }
}

//获取开机自启动状态
function getOpenAtLogin () {
  if (app.isPackaged) {
    const { openAtLogin } = app.getLoginItemSettings()
    return openAtLogin
  } else {
    const { openAtLogin } = app.getLoginItemSettings({
      path: process.execPath,
      args: [path.resolve(process.argv[1])]
    })
    return openAtLogin
  }
}