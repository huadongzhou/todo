<template>
  <div class="todo" @click.stop="addTodo" key="todo">
    <vuedraggable
      class="todo-list"
      v-model="todoList"
      v-bind="dragOptions"
      @start.stop="drag = true"
      @end.stop="drag = false"
    >
      <transition-group type="transition" :name="!drag ? 'flip-list' : null">
        <div
          class="todo-li"
          v-for="(item,index) in todoList"
          :key="item.id"
          @dblclick.stop="done($event,item,index)"
          @click.stop="edit(item)"
        >
          <p v-if="item.id != editId">{{index + 1}}.&nbsp; {{item.content }}</p>
          <div class="edit" v-else>
            <input
              v-model="item.content"
              v-focus
              @click.stop="()=>{return false;}"
              @keyup.esc="cancel(item)"
              @keyup.enter="edited(item)"
              spellcheck="false"
            />
            <i class="iconfont icon-select" @click.stop="edited(item)"></i>
            <i class="iconfont icon-close" @click.stop="cancel(item)"></i>
          </div>
        </div>
      </transition-group>
    </vuedraggable>
  </div>
</template>

<script>
import DB from '@/db'
import { defineComponent } from "vue"
import vuedraggable from 'vuedraggable'

import { getDateTime, deepClone } from '@/utils'
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
  data () {
    return {
      todoList: [
        { id: 1, content: 'kaifa' }
      ],
      drag: false,
      editId: '',
      editContent: '',
      editing: false,
      cancelClick: false,
      timer: null,
    }
  },
  created () {
    // DB = await import('@/db')
    console.log(DB)
    this.getTodoList()
  },
  methods: {
    getTodoList () {
      this.todoList = DB.get("todoList") || []
      console.log('数据', this.todoList)
    },
    addTodo () {
      if (this.editId != 'add') {
        let date = getDateTime(true)
        this.todoList.push({
          todoDate: date.split(' ')[0],
          todoDateTime: date,
          content: '',
          id: 'add',
        })
        this.editing = true
        this.editId = 'add'
        console.log('添加成功')
      }
    },
    edit (item) {
      this.timer = setTimeout(() => {
        //不取消点击事件
        if (!this.cancelClick) {
          if (this.editing && this.editId == 'add') {
            this.todoList.pop()
          }
          //进入初始化
          this.editing = true
          this.editId = item.id
          this.editContent = item.content
          this.cancelClick = false
          console.log('编辑成功')
        }
      }, 250)
    },
    edited (item) {
      if (item.content == '') {
        console.log('当前内容为空')
        return
      }
      this.editing = false
      //找到数据
      if (item.id == 'add') {
        const data = deepClone(item)
        delete data.id
        let result = DB.insert('todoList', data)
        console.log('添加完成', result)
      } else {
        let result = DB.update('todoList', { id: item.id }, item)
        console.log('更新完成', result)
      }
      //更新数据
      this.clear()
      this.getTodoList()
    },
    done (event, item, index) {
      if (item.id == 'add') return
      //执行双击事件后 禁止单击事件触发
      this.cancelClick = true
      setTimeout(() => {
        this.dblclick = false
      }, 500)
      //添加到done表
      let date = getDateTime(true)
      let data = Object.assign({ doneDate: date.split(' ')[0], doneDateTime: date }, item)

      DB.insert('doneList', data)
      //彩蛋
      CursorSpecialEffects.handleMouseDown(event)
      //清理当前表
      DB.removeById('todoList', item.id)
      this.todoList.splice(index, 1)
      //更新
      this.getTodoList()
      //完成初始化
      this.clear()
      console.log('完成')
    },
    cancel (item) {
      //取消初始化
      if (item.id == 'add') {
        this.todoList.pop()
      } else {
        item.content = this.editContent
      }
      this.editing = false
      this.editId = ''
      this.editContent = ''
      console.log('取消成功')
    },
    clear () {
      this.editId = ''
      this.editContent = ''
    }
  }
})
</script>

<style lang="scss" scoped>
.todo {
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  padding: 0 15px 30px 15px;
  .todo-list {
    .todo-li {
      height: 28px;
      p {
        width: 100%;
        height: 100%;
        line-height: 28px;
        white-space: nowrap;
        text-overflow: ellipsis;
        overflow: hidden;
        cursor: pointer;
        user-select: none;
        font-size: 14px;
      }
      .edit {
        display: flex;
        width: 100%;
        height: 100%;
        justify-content: space-between;
        input {
          flex: 1;
          height: 100%;
          outline: none;
          border: none;
          background: transparent;
          font-size: 16px;
          line-height: 28px;
        }
        i {
          line-height: 28px;
          padding: 0 5px;
          cursor: pointer;
        }
      }
      &:hover p {
        color: rgba($color: #ffffff, $alpha: 0.6);
      }
    }
  }
}
.flip-list-move {
  transition: transform 0.5s;
}
.ghost {
  opacity: 0.5;
}
</style>


