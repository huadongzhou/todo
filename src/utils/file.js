let { app } = require('electron')
let fs = require('fs')
const { exec, spawn } = require('child_process')
let path = require('path')

import log from './log'

//检查文件夹
export function checkDir (dirPath) {
  return new Promise((resolve, reject) => {
    fs.access(dirPath, (err => {
      if (err) {
        log('err', err)
        log('当前文件夹不存在：' + dirPath)
        fs.mkdirSync(dirPath, (mkErr => {
          if (mkErr) {
            console.log('mkErr', mkErr)
            log('当前文件夹创建失败：' + dirPath)
          }
          log('当前文件夹已存在：' + dirPath)
          resolve()
        }))
      }
      resolve()
    }))
  })
}
//执行bat命令文件
export function execShell (path, exePath) {
  return new Promise((resolve, reject) => {
    let bat = spawn(path)

    bat.stdout.on('data', (data) => {
      log('命令执行完成', data.toString())
    })

    bat.stderr.on('data', (data) => {
      log('err: 命令执行错误', data.toString())
      resolve()
    })

    bat.on('exit', (code) => {
      log(`子进程退出，退出码 ${code}`)
      //执行完成 重启客户端
      app.relaunch({ args: process.argv.slice(1).concat(['--relaunch']) })
      app.exit(0)

      resolve()
    })
  })

}
//执行exe文件
export function execExe (path) {
  log('exe opening: ' + path)
  setTimeout(() => {
    exec(path, (err, data) => {
      if (err) {
        log('exe open failed', err)
        return
      }
      let count = 0
      let timer = null
      timer = setInterval(() => {
        count++
        if (count > 5) {
          clearInterval(timer)
        } else {
          log('exe opening: ' + path)
        }
      }, 1000)
      log('exe open success', data.toString(), err)
    })
  })

}