<template>
  <div class="app-container">
    <Update
      v-if="showVersion"
      :versionUpdate="versionUpdate"
      :version="version"
      @updated="showVersion = false"
    />
    <router-view />
  </div>
</template>

<script>
import { defineComponent } from 'vue'
import { ipcRenderer } from 'electron'
import { mapState } from 'vuex'
import Update from '@/views/update'
export default defineComponent({
  name: 'APP',
  components: {
    Update
  },
  computed: {
    ...mapState([
      'defaultCheckUp'
    ])
  },
  watch: {
    defaultCheckUp (nVal, val) {
      console.log('检测开锁状态  准备检查更新', !this.autoCheck, nVal)
      if (!this.autoCheck && nVal) {
        this.checkVersion(true)
      }
    }
  },
  data () {
    return {
      autoCheck: false,
      showVersion: false,
      versionUpdate: 0, //0 无更新 1 热更新 2 全量更新 
      version: {
        local: '',
        online: ''
      }
    }
  },
  async created () {
    let self = this
    //不影响体验  未解锁不检查更新
    if (this.defaultCheckUp) {
      self.checkVersion(true)
    }
    //监听 版本检查
    ipcRenderer.on('checkVersion', (event, value) => {
      console.log('渲染进程监听事件-检查更新')
      self.checkVersion()
    })
  },
  methods: {
    async checkVersion (init) {
      this.autoCheck = true
      //请求监听版本
      let version = await ipcRenderer.invoke('checkVersion')
      //当没有在服务器找到版本
      if (!version) {
        this.versionUpdate = 0
        this.showVersion = true
        return
      }
      this.version.local = version.local.join('.')
      this.version.online = version.online.join('.')
      //非热更新事项 不处理
      if (version.online[0] > version.local[0] || version.online[1] > version.local[1]) {
        this.versionUpdate = 2
        this.showVersion = true
        //携带全量更新提示
      } else if (version.online[2] == version.local[2]) {
        if (!init) {
          this.showVersion = true
        }
        this.versionUpdate = 0
      } else {
        //携带热更新提示
        this.versionUpdate = 1
        this.showVersion = true
      }
      console.log(version, this.versionUpdate)
    }
  }
})
</script>

<style >
.app-container {
  width: 100%;
  height: 100%;
  position: relative;
}
</style>