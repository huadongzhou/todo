<template>
  <div>
    <h2>{{test}}</h2>
  </div>
</template>

<script>
import { defineComponent } from 'vue'
export default {
  name: 'DoneList',
  data () {
    return {
      test: 'ce'
    }
  },


}
</script>

<style lang="scss" scoped>
li {
  margin: 20px;
}
</style>


