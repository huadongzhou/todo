import { createStore } from 'vuex'

export default createStore({
  state: {
    sketch: '1',  //速记状态
    defaultCheckUp: false, //默认检查更新
  },
  mutations: {
    SETSKETCH (state, paylod) {
      state.sketch = paylod
    },
    SETEASYSTATE (state, paylod) {
      state[paylod.key] = paylod.value
    },
  },
  actions: {
    setSketch ({ commit }, paylod) {
      console.log('数据更新', paylod)
      commit('SETSKETCH', paylod)
    },
    setEasyState ({ commit }, paylod) {
      console.log('简单值的状态变更', paylod)
      commit('SETEASYSTATE', paylod)
    },
  },
  modules: {
  }
})
