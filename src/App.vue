<template>
  <div class="app-container" :class="{ unfocused: ignoreMouse,autoDisplay:autoDisplay }">
    <!-- 遮挡层 处理鼠标事件 -->
    <div class="mask"></div>
    <div class="todo-nav">
      <div class="todo-tabs">
        <router-link draggable="false" to="/">Todo</router-link>|
        <router-link draggable="false" to="/doneList">Done</router-link>
      </div>
      <div class="todo-tools">
        <transform-group name="fade" mode="out-in">
          <i class="iconfont icon-date" key="date" @click="$router.push('/sketch')"></i>
          <i v-if="sketch == '1'" class="iconfont icon-bianji" key="bianji" @click="goSketch"></i>
          <i
            v-if="sketch == '2'"
            class="iconfont icon-create"
            key="create"
            @click="$store.dispatch('setSketch','2-')"
          ></i>
          <i
            v-if="sketch == '3'"
            class="iconfont icon-save"
            key="save"
            @click="$store.dispatch('setSketch','3-')"
          ></i>
          <i class="iconfont icon-eye-close" key="close" @click="hideWindow"></i>
          <i
            :class="['iconfont', ignoreMouse ? 'icon-lock' : 'icon-unlock']"
            key="lock"
            @mouseenter="setIgnoreMouseEvents(false)"
            @mouseleave="setIgnoreMouseEvents(ignoreMouse)"
            @click="lockClick"
          ></i>
        </transform-group>
      </div>
    </div>

    <div class="todo-content scrollbar scrollbar-y">
      <transition name="fade-transform" mode="out-in">
        <router-view />
      </transition>
    </div>
  </div>
</template>

<script>
import { ipcRenderer } from 'electron'
import { mapState } from 'vuex'

export default {
  data () {
    return {
      ignoreMouse: true,
      autoDisplay: false,
    }
  },
  computed: {
    ...mapState([
      'sketch'
    ])
  },
  methods: {
    setIgnoreMouseEvents (ignore) {
      if (this.ignoreMouse) {
        this.autoDisplay = !ignore
      }
      ipcRenderer.invoke("setIgnoreMouseEvents", ignore)
    },
    hideWindow () {
      ipcRenderer.invoke("hideWindow")
    },
    lockClick () {
      this.ignoreMouse = !this.ignoreMouse
      this.autoDisplay = false
    },
    goSketch () {
      this.$router.push('/sketch')
      this.$store.dispatch('setSketch', '2')
    }
  }
}
</script>

<style lang="scss">
.app-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background-color: rgba($color: #000000, $alpha: 0.4);
  border-radius: 5px;
  .mask {
    display: none;
    position: absolute;
    z-index: 999;
    width: 100%;
    height: 100%;
  }
  .todo-nav {
    height: 26px;
    padding: 10px 20px;
    display: flex;
    justify-content: space-between;
    color: #cccccc;
    user-select: none;
    .todo-tabs {
      a {
        margin-right: 3px;
        font-weight: bold;
        color: #cccccc;
        text-decoration: none;
        &.router-link-exact-active {
          font-size: 20px;
          color: #ffffff;
        }
        &:hover {
          color: rgba($color: #ffffff, $alpha: 0.6);
        }
      }
    }
    .todo-tools {
      display: flex;
      i {
        font-size: 20px;
        line-height: 26px;
        padding: 0 5px;
        cursor: pointer;
      }
    }
  }
  .todo-content {
    flex: 1;
    margin: 10px 0;
    overflow-y: auto;
    &:hover::-webkit-scrollbar-thumb {
      display: block;
    }
  }
}
.app-container {
  &.unfocused {
    opacity: 0.1;
    background-color: rgba($color: #000000, $alpha: 0.2);
    .mask {
      display: block;
    }
    .todo-nav {
      z-index: 10000;
    }
  }
  &.autoDisplay {
    opacity: 0.3;
    background-color: rgba($color: #000000, $alpha: 0.3);
  }
}
</style>
