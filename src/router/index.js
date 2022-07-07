import { createRouter, createWebHashHistory } from 'vue-router'
import todoList from '../views/todoList'

const routes = [
  {
    path: '/',
    name: 'index',
    component: todoList
  },
  {
    path: '/todoList',
    name: 'TodoList',
    component: todoList
  },
  {
    path: '/doneList',
    name: 'DoneList',
    component: () => import('../views/doneList.vue')
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

export default router
