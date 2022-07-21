const axios = require('axios')
const fs = require('fs')
const path = require('path')


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
      // console.console.log(res.data)
      //创建写入流   asar文件不能直接生成 这里采用未分配格式
      let appFile = fs.createWriteStream(path.join(downloadPath, 'app.asar'))
      //捅进去
      res.data.pipe(appFile)
      //结束下载

      appFile.on('finish', () => {
        console.log('文件下载完毕：' + appName)
        resolve()
      })
      appFile.on('error', (err) => {
        console.log('文件下载失败：' + err)
        reject()
      })

    }).catch(err => {
      console.log('err', err)
    })
  })
}
downHotVersion('1.1.4', 'D:\\selfApplication\\Todo\\Todoing\\resources')