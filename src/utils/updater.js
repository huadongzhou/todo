
const { app, dialog, Notification } = require('electron')
import { autoUpdater } from 'electron-updater'
const axios = require('axios')
const path = require('path')
const YMAL = require('yamljs')
const fs = require('fs')

import log from './log'
const { checkDir, execShell } = require('./file')

let pgk = require('../../package.json')

//检查版本状况
export function checkVersion (rootDir) {

  let downDir = path.join(rootDir, 'Download')
  axios({
    url: 'http://139.159.242.135/latest.yml',
    method: 'GET'
  }).then(result => {
    //拿到文件版本号  第一位全新版本  第二位进程调整 第三位业务调整
    let data = YMAL.parse(result.data)
    // log('读取结果', data)
    let onlineVersion = data.version.split('.')
    let localVersion = app.getVersion().split('.')
    log('version', localVersion, onlineVersion)
    //非热更新事项 不处理
    if (onlineVersion[0] > localVersion[0] || onlineVersion[1] > localVersion[1]) {
      //大版本自动更新
      const log = require("electron-log")
      log.transports.file.level = "debug"
      autoUpdater.logger = log
      autoUpdater.checkForUpdatesAndNotify()
    } else if (onlineVersion[2] == localVersion[2]) {
      return false
    } else {
      //热更新业务
      const dialogOpts = {
        type: 'info',
        buttons: ['立即更新', '稍后更新'],
        title: '更新提醒',
        message: `您有新的更新!`,
        detail: `内容如下：` + `${data.version}`
      }
      dialog.showMessageBox(dialogOpts).then((returnValue) => {
        log(returnValue)
        if (returnValue.response === 0) {
          try {
            //检查文件夹
            checkDir(downDir).then(async () => {
              if (Notification.isSupported()) {
                let notification = new Notification({
                  title: '提示',
                  body: '当前应用处于更新中，请不要退出当前应用！',
                  silent: true,
                })
                notification.show()
              }
              //下载文件内容
              await downHotVersion(data.version, downDir)
              //修改本地版本
              changeVersion(data.version)
              // //复制文件脚本
              log('复制脚本位置：', path.join(rootDir, 'copy.bat'))
              await execShell(path.join(rootDir, 'copy.bat'))
              if (Notification.isSupported()) {
                let notification = new Notification({
                  title: '提示',
                  body: '当前应用更新完成,重启中...',
                  silent: true,
                })
                notification.show()
              }
            })
          } catch (err) {
            log(err)
          }
        }
      })
    }
  })
}
//改变本地版本信息
function changeVersion (name) {
  return new Promise((resolve, reject) => {
    pgk.version = name
    log('version change success!   ' + pgk.version)
    resolve()
  })

}
// log('version !   ' + pgk.version)
//下载新内容
function downHotVersion (version, downloadPath) {
  let appName = 'app-' + version + '.asar'
  let url = 'http://139.159.242.135/hot/' + appName
  return new Promise((resolve, reject) => {
    axios({
      url,
      method: 'GET',
      timeout: 10 * 60 * 1000,
      maxContentLength: Infinity,
      responseType: 'stream',
    }).then(res => {
      // console.log(res.data)
      //创建写入流   asar文件不能直接生成 这里采用未分配格式
      let appFile = fs.createWriteStream(path.join(downloadPath, 'app'))
      //捅进去
      res.data.pipe(appFile)
      //结束下载

      appFile.on('finish', () => {
        log('文件下载完毕：' + appName)
        resolve()
      })
      appFile.on('error', (err) => {
        log('文件下载失败：' + err)
        reject()
      })

    }).catch(err => {
      log('err', err)
    })
  })
}
// console.log(fs)
//downHotVersion('1.1.4', 'D:\\Code\\clientApp\\Electron\\electron-vue3\\dist_electron\\Download\\')