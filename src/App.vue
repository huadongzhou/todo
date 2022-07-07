<template>
  <div id="app-container">
    <!-- 遮挡层 处理鼠标事件 -->
    <div class="mask"></div>
    <div class="todo-nav">
      <div class="todo-tabs">
        <router-link draggable="false" to="/todoList">Todo</router-link>
        <router-link draggable="false" to="/doneList">Done</router-link>
      </div>
      <div class="todo-tools">
        <transform-group name="fade" mode="out-in">
          <i class="iconfont icon-eye-close" key="hide" @click="hideWindow"></i>
          <i
            :class="['iconfont', ignoreMouse ? 'icon-lock' : 'icon-unlock']"
            key="lock"
            @mouseenter="setIgnoreMouseEvents(false)"
            @mouseleave="setIgnoreMouseEvents(ignoreMouse)"
            @click="ignoreMouse = !ignoreMouse"
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
import { defineComponent } from 'vue'
import { ipcRenderer } from 'electron'

export default defineComponent({
  data () {
    return {
      ignoreMouse: false,
    }
  },
  methods: {
    setIgnoreMouseEvents (ignore) {
      ipcRenderer.invoke("setIgnoreMouseEvents", ignore)
    },
    hideWindow () {
      ipcRenderer.invoke("hideWindow")
    }
  }
})
</script>

<style lang="scss">
#app-container {
  display: flex;
  flex-direction: column;
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
    overflow: auto;
    &:hover::-webkit-scrollbar-thumb {
      display: block;
    }
  }
}
#app.unfocused {
  opacity: 0.8;
  .mask {
    display: block;
  }
  .tools {
    z-index: 1000;
  }
}
</style>
