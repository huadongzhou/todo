<template>
  <div class="done" key="done">
    <div class="done-list" v-for="(data,key) in doneList" :key="key">
      <div class="group">{{ getDateStr(key) }}</div>
      <div
        class="done-li"
        v-for="(item,index) in data"
        :key="item.id"
        @click.stop="editId == item.id ? (editId = '') : (editId = item.id)"
      >
        <p>{{index + 1}}.&nbsp; {{item.content}}</p>
        <i
          v-if="editId === item.id"
          class="iconfont icon-back"
          @click.stop="restore($event,data,item,index)"
        ></i>
        <i
          v-if="editId === item.id"
          class="iconfont icon-close"
          @click.stop="remove($event,data,item,index)"
        ></i>
      </div>
    </div>
  </div>
</template>

<script>
import { defineComponent } from "vue"

import DB from '@/db'
import { getDateStr } from '@/utils'
import CursorSpecialEffects from "@/utils/fireworks"

export default defineComponent({
  name: 'DoneList',
  data () {
    return {
      getDateStr,
      doneList: [
        { id: 1, content: 'kaifa' }
      ],
      drag: false,
      editId: '',
      editContent: ''
    }
  },
  created () {
    this.getDoneList()
  },
  methods: {
    getDoneList () {
      this.doneList = DB.groupby("doneList", "doneDate") || []
      console.log('数据', this.doneList)
    },
    restore (event, data, item, index) {
      console.log('数据', data, item, index)
      //添加到todo表

      DB.insert('todoList', item)
      //彩蛋
      CursorSpecialEffects.handleMouseDown(event)
      //清理当前表
      DB.removeById('doneList', item.id)
      data.splice(index, 1)
      //更新
      this.getDoneList()
      //完成初始化
      console.log('完成')
    },
    remove (event, data, item, index) {
      console.log('数据', data, item, index)

      //清理当前表
      DB.removeById('doneList', item.id)
      //彩蛋
      CursorSpecialEffects.handleMouseDown(event)
      data.splice(index, 1)
      //更新
      this.getDoneList()
      //完成初始化
      console.log('完成')
    }
  }
})
</script>

<style lang="scss" scoped>
.done {
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  padding: 0 15px 30px 15px;
  color: #fff;
  .done-list {
    .group {
      position: sticky;
      top: 0;
      z-index: -999;
      height: 188px;
      line-height: 180px;
      box-sizing: border-box;
      color: rgba($color: #cccccc, $alpha: 0.8);
      font-size: 35px;
      text-align: center;
      user-select: none;
    }
    .done-li {
      display: flex;
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
      i {
        line-height: 28px;
        padding: 0 5px;
        cursor: pointer;
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


