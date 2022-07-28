import { createStore } from 'vuex'

export default createStore({
  state: {
    sketch: '1'
  },
  mutations: {
    SETSKETCH (state, paylod) {
      state.sketch = paylod
    }
  },
  actions: {
    setSketch ({ commit }, paylod) {
      console.log('数据更新', paylod)
      commit('SETSKETCH', paylod)
    }
  },
  modules: {
  }
})
