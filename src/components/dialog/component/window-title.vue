<template>
  <div class="title">
    <!-- left -->
    <div class="title-l">
      <slot name="left"></slot>
    </div>
    <!-- right -->
    <div class="title-r">
      <slot name="right"></slot>
      <i class="iconfont icon-close" @click="$emit('close')"></i>
    </div>
  </div>
</template>

<script>
import { defineComponent } from 'vue'
export default defineComponent({
  name: 'Window-Title'
})
</script>

<style lang="scss">
.title {
  height: 30px;
  padding: 0 5px;
  line-height: 30px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #f7f7f7;
}
.title-l {
  font-size: 14px;
  line-height: 30px;
  text-indent: 10px;
  letter-spacing: 1px;
  color: #666;
}
.title-r {
  display: flex;
  i {
    font-size: 20px;
    line-height: 26px;
    padding: 0 5px;
    cursor: pointer;
  }
}
</style>