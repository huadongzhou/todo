<template>
  <div class="update-container">
    <div class="update-logo">
      <img src="@/assets/images/logo.png" alt />
    </div>
    <div class="update-tips">
      <!-- 没有更新 -->
      <div v-if="updated" class="update-tip">
        <div class="update-tip-title">Toding 已更新完成 !</div>
        <p class="update-tip-desc">准备重启应用</p>
      </div>
      <!-- 提示更新 -->
      <div v-else-if="versionUpdate == 1 || versionUpdate == 2" class="update-tip">
        <div class="update-tip-title">发现新版本{{version.online}} !</div>
        <p class="update-tip-desc">Toding 当前版本{{version.local}}</p>
        <!-- <p class="update-tip-desc">Toding {{newVersion}}准备更新</p> -->
      </div>
      <!-- 没有更新 -->
      <div v-else class="update-tip">
        <div class="update-tip-title">您使用的是最新版本</div>
        <p class="update-tip-desc">Toding {{version.local}}已是最新版本</p>
      </div>
    </div>
    <!-- 更新进度 -->
    <div class="update-version"></div>
    <!-- 按钮 -->
    <div class="update-btns">
      <div v-if="updated">
        <button @click="reloadVersion">立即重启应用</button>
        <button @click="$emit('updated')">暂不</button>
      </div>
      <div v-else-if="versionUpdate == 1 || versionUpdate == 2">
        <button :disabled="updating" @click="update">立即更新</button>
        <button :disabled="updating" @click="$emit('updated')">暂不</button>
      </div>
      <div v-else>
        <button @click="$emit('updated')">取消</button>
      </div>
    </div>
  </div>
</template>

<script>
import { defineComponent } from 'vue'
import { ipcRenderer } from 'electron'


export default defineComponent({
  name: 'Update',
  props: {
    version: {
      type: Object,
      default () {
        return {
          local: '',
          online: ''
        }
      }
    },
    versionUpdate: {
      type: Number,
      default: 0 //0 无更新 1 热更新 2 全量更新 
    }
  },
  data () {
    return {
      updated: false,
      updating: false
    }
  },
  created () {
    this.updated = false
  },
  methods: {
    async checkVersion () {
      //请求监听版本
      let version = await ipcRenderer.invoke('checkVersion')


    },
    async update () {
      this.updating = true
      if (this.versionUpdate == 1) {
        const result = await ipcRenderer.invoke('hotVersion', this.version.online)
        //更新状态
        this.updated = result
        console.log('热更新状态')
      } else if (this.versionUpdate == 2) {
        await ipcRenderer.invoke('allVersion')
      }
      this.updating = false
    },
    reloadVersion () {
      //确定重启应用
      ipcRenderer.invoke('reloadVersion')
    }
  }
})
</script>

<style  lang="scss" scoped>
.update-container {
  width: 280px;
  min-height: 120px;
  padding: 10px;
  box-sizing: border-box;
  background: #fff;
  color: #000;
  border-radius: 2px;
  box-shadow: 0 0 5px 2px #999;
  position: absolute;
  left: calc((100% - 280px) / 2);
  top: calc((100% - 120px) / 2);
  .update-logo {
    display: inline-block;
    width: 48px;
    vertical-align: top;
    img {
      width: 100%;
    }
  }
  .update-tips {
    margin-left: 10px;
    display: inline-block;
    .update-tip-title {
      font-size: 14px;
      color: #444;
      margin-bottom: 10px;
    }
    .update-tip-desc {
      font-size: 12px;
      color: #666;
    }
  }
  .update-version {
  }
  .update-btns {
    padding: 20px 0 0;
    text-align: right;
    button {
      padding: 6px 18px;
      margin-left: 10px;
      border: none;
      outline: none;
      background: #009ecb;
      border-radius: 3px;
      color: #fff;
      cursor: pointer;
      &:hover {
        transform: scale(0.97);
      }
    }
  }
}
</style>