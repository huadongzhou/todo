import { createRouter, createWebHashHistory } from 'vue-router'
import todo from '../views/todo/todo'

const routes = [
  {
    path: '/',
    name: 'todo',
    component: todo,
    redirect: '/todoList',
    children: [
      {
        path: 'todoList',
        name: 'TodoList',
        component: () => import('../views/todo/todoList.vue')
      },
      {
        path: 'doneList',
        name: 'DoneList',
        component: () => import('../views/todo/doneList.vue')
      },
      {
        path: 'sketch',
        name: 'SketchList',
        component: () => import('../views/todo/sketchList.vue')
      },
    ]
  },

]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

export default router
