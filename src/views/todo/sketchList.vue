<template>
  <div class="sketch stopSelect" @dblclick="addSketch" key="sketch">
    <!-- 编辑器 -->
    <div v-if="editing" class="sketch-editer">
      <!-- <p class="sketch-name">{{tempData.title}}</p> -->
      <input type="text" v-model="tempData.title" />
      <div class="sketch-utils">
        <i class="iconfont icon-back" key="back" @click="cancel"></i>
      </div>
      <textarea class="sketch-text" v-focus v-model="tempData.content"></textarea>
    </div>
    <vuedraggable
      v-else
      class="sketch-list"
      v-model="sketchList"
      v-bind="dragOptions"
      @start.stop="dragState(true,$event)"
      @end.stop="dragState(false,$event)"
    >
      <transition-group type="transition" :name="!drag ? 'flip-list' : null">
        <div
          class="sketch-li"
          v-for="(item,index) in sketchList"
          :key="item.id"
          @click.stop="editSketch(item)"
        >
          <div class="sketch-title">
            <span class="sketch-title-index">{{index + 1}}.&nbsp;</span>
            <p style="text-align:center">{{item.title }}</p>
          </div>
          <div class="sketch-content">
            <p class="moreline">{{item.content }}</p>
          </div>
        </div>
      </transition-group>
    </vuedraggable>
  </div>
</template>

<script>
import DB from '@/db'
import { defineComponent } from "vue"
import { mapState } from 'vuex'
import vuedraggable from '../../../node_modules/vuedraggable/src/vuedraggable'


import { getDateTime, deepClone } from '@/utils'
export default defineComponent({
  name: 'sketchList',
  components: {
    vuedraggable
  },
  directives: {
    focus: {
      mounted: function (el) {
        el.focus()
      }
    }
  },
  computed: {
    ...mapState([
      'sketch'
    ])
  },
  watch: {
    // 1ks 2
    sketch (newVal) {
      console.log('快捷键的值变化', newVal)
      if (newVal == '2-') {
        this.addSketch()
      } else if (newVal == '3-') {
        this.editedSketch()
      }
    }
  },
  data () {
    return {
      sketchList: [
        { id: 1, content: 'kaifa' }
      ],
      dragOptions: {
        animation: 200,
        group: "description",
        disabled: false,
        ghostClass: "ghost"
      },
      drag: false,
      tempData: {},
      editData: {},
      editing: false,
      cancelClick: false,
      timer: null,
    }
  },
  created () {

    this.getSketchList()
  },
  methods: {
    dragState (state, e) {
      this.drag = state
      if (!state) {
        //更新拖拽数据
        let oldIndex = e.oldIndex
        let newIndex = e.newIndex
        let oldData = this.sketchList.splice(oldIndex, 1)[0]
        //判断前后
        let index = newIndex - 1
        // console.log('index', index)
        if (index - 1 <= 0) {
          this.sketchList.unshift(oldData)
        } else {
          this.sketchList.splice(index, 0, oldData)
        }
        //更新至数据库
        DB.set('sketchList', this.sketchList)
      }
      console.log(e)
    },
    getSketchList () {
      this.sketchList = DB.get("sketchList") || []
      console.log('数据', this.sketchList)
    },
    addSketch () {
      this.$store.dispatch('setSketch', '3')
      if (this.tempData && this.tempData.id != 'add') {
        let date = getDateTime(true)
        let newSketch = {
          sketchDate: date.split(' ')[0],
          sketchDateTime: date,
          content: '',
          title: date.split(' ')[0] + '  速记',
          id: 'add',
        }
        this.sketchList.push(newSketch)
        this.editing = true
        this.tempData = newSketch
        console.log('添加成功')
      }
    },
    editSketch (item) {
      this.timer = setTimeout(() => {
        //不取消点击事件
        if (!this.cancelClick) {
          //
          this.$store.dispatch('setSketch', '3')
          if (this.editing && this.tempData.id == 'add') {
            this.sketchList.pop()
          }
          //进入初始化
          this.editing = true
          this.editData = item
          this.tempData = deepClone(item)
          this.cancelClick = false
          console.log('编辑成功')
        }
      }, 250)
    },
    editedSketch () {
      if (this.tempData.content == '') {
        console.log('当前内容为空')
        return
      } else {
        console.log(this.tempData)
      }
      this.editing = false
      //找到数据
      if (this.tempData.id == 'add') {
        delete this.tempData.id
        let result = DB.insert('sketchList', this.tempData)
        console.log('添加完成', result)
      } else {
        let result = DB.update('sketchList', { id: this.tempData.id }, this.tempData)
        console.log('更新完成', result)
      }
      //更新数据
      this.clear()
      this.getSketchList()
    },
    cancel () {
      this.$store.dispatch('setSketch', '2')
      //取消初始化
      if (this.tempData.id == 'add') {
        this.sketchList.pop()
      }
      this.editing = false
      this.tempData = {}
      this.editData = {}
      console.log('取消成功')
    },
    clear () {
      this.$store.dispatch('setSketch', '2')
      this.tempData = {}
      this.editData = {}
    }
  },
  beforeRouteLeave () {
    this.$store.dispatch('setSketch', '1')
  }
})
</script>

<style lang="scss" scoped>
.sketch {
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  padding: 0 15px 30px 15px;
  position: relative;
  color: #fff;
  .sketch-list {
    .sketch-li {
      margin: 20px 0;
      cursor: pointer;
      .sketch-title {
        position: relative;
        font-size: 14px;
        margin-bottom: 5px;
        .sketch-title-index {
          position: absolute;
          left: 0;
          top: 0;
        }
      }
      .sketch-content {
        font-size: 12px;
        text-indent: 20px;
      }
      &:hover p {
        color: rgba($color: #ffffff, $alpha: 0.6);
      }
    }
  }
  .sketch-editer {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    bottom: 0;
    // background: red;
    z-index: 999;
    padding: 0 20px;
    input {
      width: 100%;
      margin-bottom: 10px;
      outline: none;
      border: none;
      background: transparent;
      font-size: 14px;
      line-height: 28px;
      text-align: center;
    }
    .sketch-utils {
      position: absolute;
      top: 0;
      right: 20px;
      i {
        font-size: 20px;
        line-height: 26px;
        padding: 0 5px;
        cursor: pointer;
      }
    }
    .sketch-text {
      width: 100%;
      height: calc(100% - 38px);
      background: transparent;
      border: none;
      outline: none;
      resize: none;
      overflow-y: hidden;
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


