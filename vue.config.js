// vue.config.js

module.exports = {
  pluginOptions: {
    electronBuilder: {
      builderOptions: {
        "appId": "com.electron.zhd",
        productName: 'Todoing',
        copyright: 'Copyright @2022 zhd',
        directories: {
          output: 'build'
        },
        compression: 'store',
        "win": {
          icon: 'public/logo.ico',
          // requestedExecutionLevel: "highestAvailable",// highestAvailable 已有最高权限
          "target": {
            target: "nsis",
            arch: ['ia32'],
            // "arch": [
            //   // 这个意思是打出来32 bit + 64 bit的包，但是要注意：这样打包出来的安装包体积比较大，所以建议直接打32的安装包。
            //   // arch “x64” | “ia32” | “armv7l” | “arm64”> | “x64” | “ia32” | “armv7l” | “arm64”  -arch支持列表
            //   "x64",
            //   "ia32"
            // ],
          },
          "publish": {
            "provider": "generic",
            "url": "http://139.159.242.135/"
          }
        },
        "nsis": {
          "artifactName": "${productName}-${version}.${ext}",
          "oneClick": false,
          "perMachine": true,
          "allowToChangeInstallationDirectory": true,
          "installerIcon": "public/logo.ico",// 安装图标
          "uninstallerIcon": "public/logo.ico",//卸载图标
          "installerHeaderIcon": "public/logo.ico", // 安装时头部图标
          "deleteAppDataOnUninstall": false, //卸载时时候会删除用户数据
        },
        "extraFiles": ['copy.bat'],
        // "nsis": {
        //   "oneClick": false, // 是否一键安装
        //   "allowElevation": true, // 允许请求提升。 如果为false，则用户必须使用提升的权限重新启动安装程序。
        //   "allowToChangeInstallationDirectory": true, // 允许修改安装目录
        //   "installerIcon": "./build/icons/aaa.ico",// 安装图标
        //   "uninstallerIcon": "./build/icons/bbb.ico",//卸载图标
        //   "installerHeaderIcon": "./build/icons/aaa.ico", // 安装时头部图标
        //   "createDesktopShortcut": true, // 创建桌面图标
        //   "createStartMenuShortcut": true,// 创建开始菜单图标
        //   "shortcutName": "xxxx", // 图标名称
        //   "include": "build/script/installer.nsh", // 包含的自定义nsis脚本 这个对于构建需求严格得安装过程相当有用。
        //   "script" : "build/script/installer.nsh" // NSIS脚本的路径，用于自定义安装程序。 默认为build / installer.nsi  
        // },
      },
      nodeIntegration: true
    }
  },
  configureWebpack: {
    externals: {}
  }
}
