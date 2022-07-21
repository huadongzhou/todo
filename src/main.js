import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import store from './store'

import DB from './db'
import './styles/index.scss'
import "./assets/iconfont/iconfont.css"

//初始化数据库
DB.init().then(() => {

  const app = createApp(App).use(router).use(store)
  // 屏蔽警告信息
  // app.config.warnHandler = () => null

  app.mount('#app')

})
