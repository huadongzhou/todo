<template>
  <div class="todo">
    <vuedraggable
      class="todo-list"
      v-model="todoList"
      v-bind="dragOptions"
      @start="drag = true"
      @end="drag = false"
      :disabled="editIndex !== -1"
    >
      <transition-group type="transition" :name="!drag ? 'flip-list' : null">
        <div
          class="item"
          v-for="(item,index) in todoList"
          :key="item.id"
          @dblclick.stop="done($event,item)"
          @click.stop="edit(item)"
        >
          <p v-if="item.id != editId">{{index + 1}}.&nbsp; {{item.content}}</p>
          <div class="edit" v-else>
            <input
              v-model="todo.content"
              v-focus
              @click.stop="()=>{return false;}"
              @keyup.esc="cancel(item)"
              @keyup.enter="edited(item)"
              spellcheck="false"
            />
            <i class="iconfont icon-select" @click.stop="edited"></i>
            <i class="iconfont icon-close" @click.stop="clear(item)"></i>
          </div>
        </div>
      </transition-group>
    </vuedraggable>
  </div>
</template>

<script>
import { defineComponent } from "vue"
import { ipcRenderer } from "electron"
import vuedraggable from 'vuedraggable'

import DB from '../utils/db'
import CursorSpecialEffects from "@/utils/fireworks"

export default defineComponent({
  name: 'TodoList',
  components: [vuedraggable],
  directives: {
    focus: {
      inserted: function (el) {
        el.focus()
      }
    }
  },
  computed: {
    dragOptions () {
      return {
        animation: 200,
        group: "description",
        disabled: false,
        ghostClass: "ghost",
      }
    },
  },
  data () {
    return {
      todoList: [
        { id: 1, content: 'kaifa' }
      ],
      editId: '',
      editContent: ''
    }
  },
  created () {
    //获取数据
    this.getTodoList()
  },
  methods: {
    getTodoList () {
      this.todoList = DB.get("todoList") || []
      console.log('数据', this.todoList)
    },
    edit (item) {
      //进入初始化
      this.editId = item.id
      this.editContent = item.content
    },
    edited (item) {
      //找到数据

      //更新数据
    },
    done (item) {
      //完成初始化
      this.editId = ''
      this.editContent = ''
    },
    cancel (item) {
      //取消初始化
      item.content = this.editContent
      this.editId = ''
    },
    clear (item) {

    }
  }
})
</script>

<style lang="scss" scoped>
</style>


